import { atom, read, update } from 'claude-code'
import type { EngineInterface, Register } from 'claude-code'

import type { ArmState, RunState, TestRow } from '../types'

const PANE = 'pvc-test'
const SCRIPT = 'pilot/owner-test/run.py'
const run = atom({ plugin: 'pvc-test', key: 'run' } as const, { phase: 'idle', rows: [], log: [] } as RunState)

const USAGE = [
  'Usage: /pvc-test [all | T1 T2 ...] [--model <model>] [--eval-model <model>] [--budget <usd>]',
  '       /pvc-test results    (open the results pane)',
  'Tests: T1 Hello + Save, T2 Profile card, T3 Counter, T4 Sign-up form, T5 Wizard, T6 Dashboard.',
  'Start Claude Code in the Project-VectorCanvas folder first. Needs python3, cargo and the claude command.',
].join('\n')

type Event = {
  event: string
  text?: string
  test?: string
  arm?: 'pvc' | 'control' | 'evaluator'
  ok?: boolean
  run_dir?: string
  report?: string
  error?: string
  tests?: { id: string; title: string; level: number }[]
  result?: {
    pvc_total?: number
    control_total?: number
    pvc_passed?: number
    control_passed?: number
    of?: number
    control_language?: string
    pvc_rendered?: boolean
    evaluator_ok?: boolean
  }
}

function setRow(state: RunState, id: string, change: Partial<TestRow>): RunState {
  return { ...state, rows: state.rows.map(r => (r.id === id ? { ...r, ...change } : r)) }
}

function apply(state: RunState, e: Event): RunState {
  const log = e.text ? [...state.log, e.text].slice(-8) : state.log
  const s = { ...state, log }
  switch (e.event) {
    case 'start':
      return {
        ...s,
        phase: 'building',
        runDir: e.run_dir,
        rows: (e.tests ?? []).map(t => ({ ...t, pvc: 'waiting', control: 'waiting', evaluator: 'waiting' })),
      }
    case 'agent_start':
      return e.test && e.arm ? setRow({ ...s, phase: 'running' }, e.test, { [e.arm]: 'running' as ArmState }) : s
    case 'agent_done':
      return e.test && e.arm ? setRow(s, e.test, { [e.arm]: (e.ok ? 'done' : 'failed') as ArmState }) : s
    case 'test_done': {
      const r = e.result ?? {}
      return e.test
        ? setRow(s, e.test, {
            evaluator: r.evaluator_ok ? 'done' : 'failed',
            pvcTotal: r.pvc_total,
            controlTotal: r.control_total,
            pvcPassed: r.pvc_passed,
            controlPassed: r.control_passed,
            of: r.of,
            controlLanguage: r.control_language,
            pvcRendered: r.pvc_rendered,
          })
        : s
    }
    case 'test_error':
      return e.test ? setRow(s, e.test, { error: e.error }) : s
    case 'done':
      return { ...s, phase: 'done', report: e.report }
    default:
      return s
  }
}

function parseArgs(args: string): { argv: string[] } | { error: string } {
  const words = args.trim().split(/\s+/).filter(Boolean)
  const out: string[] = []
  for (let i = 0; i < words.length; i++) {
    const w = words[i] ?? ''
    if (w === 'all') continue
    if (w === '--model' || w === '--eval-model' || w === '--budget') {
      const v = words[++i]
      if (!v) return { error: `${w} needs a value.` }
      out.push(w, v)
    } else if (/^t[1-6]$/i.test(w)) {
      out.push(w.toUpperCase())
    } else {
      return { error: `Unknown argument: ${w}` }
    }
  }
  return { argv: out }
}

async function startRun($: EngineInterface, extra: string[]) {
  let buffer = ''
  const onLine = async (line: string) => {
    if (!line.trim()) return
    let ev: Event
    try {
      ev = JSON.parse(line)
    } catch {
      ev = { event: 'log', text: line.slice(0, 300) }
    }
    await update($, run, s => apply(s, ev))
    if (ev.event === 'test_done' && ev.test) void $.ui.toast(`PVC test ${ev.test} finished`)
  }
  let code: number | null = null
  let stderr = ''
  for (const python of ['python3', 'python']) {
    try {
      const child = $.process.spawn({ argv: [python, SCRIPT, '--events', ...extra] })
      for await (const piece of child) {
        if (piece.stream === 'stderr') {
          stderr = (stderr + piece.text).slice(-2000)
          continue
        }
        buffer += piece.text
        let nl = buffer.indexOf('\n')
        while (nl >= 0) {
          await onLine(buffer.slice(0, nl))
          buffer = buffer.slice(nl + 1)
          nl = buffer.indexOf('\n')
        }
      }
      await onLine(buffer)
      code = 0
      break
    } catch (err) {
      stderr = String(err)
    }
  }
  const final = await read($, run)
  if (final.phase !== 'done') {
    await update($, run, (s): RunState => ({ ...s, phase: 'failed', error: stderr.trim().split('\n').slice(-6).join('\n') || 'The test script stopped.' }))
    void $.ui.toast('PVC test stopped with an error. See the pvc-test pane.')
  } else {
    void $.ui.notify(`Report: ${final.report}`, { title: 'PVC test finished' })
  }
  return code
}

const MARK: Record<ArmState, string> = { waiting: '·', running: '…', done: '✓', failed: '✗' }

function pad(s: string, n: number) {
  return s.length >= n ? s.slice(0, n) : s + ' '.repeat(n - s.length)
}

function num(n?: number) {
  return n === undefined ? '—' : n.toLocaleString('en-US')
}

export const register: Register = on => {
  on('session.start', async ($, e, next) => {
    await $.command.register({
      name: 'pvc-test',
      description: 'Run the PVC A/B test (PVC agent vs control agent, judged by an evaluator agent)',
      argumentHint: '[all | T1..T6 | results] [--model <model>]',
    })
    return next(e)
  })

  on('command.run', { command: 'pvc-test' }, async ($, e) => {
    const args = e.args.trim()
    if (args === 'help') return { text: USAGE }
    if (args === 'results') {
      await $.ui.open({ id: PANE, title: 'PVC test' })
      return { text: 'Opened the PVC test pane.' }
    }
    const state = await read($, run)
    if (state.phase === 'building' || state.phase === 'running') {
      await $.ui.open({ id: PANE, title: 'PVC test' })
      return { text: 'A PVC test run is already in progress. Its progress is in the pane.' }
    }
    const parsed = parseArgs(args)
    if ('error' in parsed) return { text: `${parsed.error}\n\n${USAGE}` }
    const found = await $.fs.stat(SCRIPT).catch(() => undefined)
    if (!found) {
      return { text: `Cannot find ${SCRIPT}. Start Claude Code in the Project-VectorCanvas folder, then run /pvc-test again.` }
    }
    await update($, run, (): RunState => ({ phase: 'building', rows: [], log: ['Starting…'] }))
    $.clock.after(0, () => {
      void startRun($, parsed.argv)
    })
    await $.ui.open({ id: PANE, title: 'PVC test' })
    return {
      text:
        'PVC test started. It builds the PVC tools, then runs a PVC agent, a control agent and an evaluator for each test. ' +
        'This takes several minutes per test. Progress and results are in the PVC test pane (/pvc-test results to reopen it).',
    }
  })

  on('ui.render', { component: 'Pane', requestId: PANE }, async ($, e) => {
    const { Box, Text } = $.ui.resolve(e)
    const s = await read($, run)
    const head = `${pad('Test', 22)} ${pad('PVC', 4)} ${pad('Ctl', 4)} ${pad('Eval', 5)} ${pad('PVC tokens', 11)} ${pad('Ctl tokens', 11)} ${pad('PVC ✓', 6)} ${pad('Ctl ✓', 6)} Ctl language`
    const phase: Record<RunState['phase'], string> = {
      idle: 'No run yet. Type /pvc-test to start one.',
      building: 'Building the PVC tools…',
      running: 'Running…',
      done: 'Finished.',
      failed: 'Stopped with an error.',
    }
    return (
      <Box flexDirection="column">
        <Text bold>{phase[s.phase]}</Text>
        {s.rows.length > 0 && <Text dimColor>{head}</Text>}
        {s.rows.map(r => (
          <Text key={r.id}>
            {pad(`${r.id} ${r.title}`, 22)} {pad(MARK[r.pvc], 4)} {pad(MARK[r.control], 4)} {pad(MARK[r.evaluator], 5)}{' '}
            {pad(num(r.pvcTotal), 11)} {pad(num(r.controlTotal), 11)}{' '}
            {pad(r.pvcPassed === undefined ? '—' : `${r.pvcPassed}/${r.of}`, 6)}{' '}
            {pad(r.controlPassed === undefined ? '—' : `${r.controlPassed}/${r.of}`, 6)} {r.controlLanguage ?? '—'}
            {r.pvcRendered === false ? '  (PVC not drawn)' : ''}
            {r.error ? `  error: ${r.error}` : ''}
          </Text>
        ))}
        {s.error && <Text color="red">{s.error}</Text>}
        {s.report && <Text>Full report: {s.report}</Text>}
        {s.log.length > 0 && <Text dimColor>{s.log.slice(-3).join('\n')}</Text>}
      </Box>
    )
  })
}
