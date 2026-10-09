export type ArmState = 'waiting' | 'running' | 'done' | 'failed'

export type TestRow = {
  id: string
  title: string
  level: number
  pvc: ArmState
  control: ArmState
  evaluator: ArmState
  pvcTotal?: number
  controlTotal?: number
  pvcPassed?: number
  controlPassed?: number
  of?: number
  controlLanguage?: string
  pvcRendered?: boolean
  file?: string
  error?: string
}

export type RunState = {
  phase: 'idle' | 'building' | 'running' | 'done' | 'failed'
  runDir?: string
  report?: string
  rows: TestRow[]
  log: string[]
  error?: string
}

declare module 'claude-code' {
  interface PluginState {
    'pvc-test': { run: RunState }
  }
}
