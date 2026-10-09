import { describe, expect, test } from 'claude-code/testing'

type Run = (input: { command: string; args: string }) => Promise<{ text?: string }>

describe('/pvc-test', () => {
  test('help prints the usage', async $ => {
    const r = await ($.command.run as unknown as Run)({ command: 'pvc-test', args: 'help' })
    expect(r.text).toContain('Usage: /pvc-test')
  })

  test('an unknown argument is refused with the usage', async $ => {
    const r = await ($.command.run as unknown as Run)({ command: 'pvc-test', args: 'T9' })
    expect(r.text).toContain('Unknown argument: T9')
    expect(r.text).toContain('Usage: /pvc-test')
  })

  test('a flag without a value is refused', async $ => {
    const r = await ($.command.run as unknown as Run)({ command: 'pvc-test', args: 'T1 --model' })
    expect(r.text).toContain('--model needs a value.')
  })
})
