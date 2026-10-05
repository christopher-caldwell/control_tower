import { describe, expect, it } from 'vitest'

import { parseSnapshot } from '@/features/workflows/execution/api/snapshot'

const snapshot = {
  workspace_name: 'workspace',
  workflow: { id: 'fixture', name: 'Fixture' },
  current_status: 'available',
  status_issue: null,
  checkpoint: {
    accepted_stage: { number: 10, name: 'seed' },
    pending_transition: null,
    state: { completed_stage_count: 1, uuid: null, pending: null },
  },
  movement_choices: [{ direction: 'up', target_stage: 200 }],
  stages: [
    {
      number: 10,
      name: 'seed',
      state: 'accepted',
      is_accepted_checkpoint: true,
      definitions: [{ role: 'up', path: 'stages/010-seed/up' }],
    },
  ],
  movement_busy: false,
  observation: {
    direction: 'up',
    target_stage: 200,
    state: 'complete',
    active_role: null,
    role_results: [],
    failure: null,
    verification_choices: null,
  },
}

const encodeSnapshot = (value: unknown): string => JSON.stringify(value)

describe('workflow snapshot parser', () => {
  it('returns a typed snapshot with sparse stage identities intact', () => {
    const result = parseSnapshot(encodeSnapshot(snapshot), 'fixture')
    expect(result.workflow.id).toBe('fixture')
    expect(result.stages[0]?.number).toBe(10)
    expect(result.movement_choices[0]?.target_stage).toBe(200)
  })

  it('reports the path of an invalid nested result list', () => {
    const malformed = { ...snapshot, observation: { ...snapshot.observation, role_results: null } }
    expect(() => parseSnapshot(encodeSnapshot(malformed), 'fixture')).toThrow('observation.role_results')
  })

  it('rejects a snapshot delivered on a different workflow subscription', () => {
    expect(() => parseSnapshot(encodeSnapshot(snapshot), 'other')).toThrow('workflow.id')
  })

  it('rejects JSON syntax errors with a readable location', () => {
    expect(() => parseSnapshot('{invalid', 'fixture')).toThrow('workflow snapshot at JSON')
  })
})
