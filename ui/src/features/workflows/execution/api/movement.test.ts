import { afterEach, describe, expect, it, vi } from 'vitest'

import { ApiFailure } from '@/api/client'
import type { CheckpointState, MovementChoice } from '@/api/types'
import { submitWorkflowMovement } from '@/features/workflows/execution/api/options'

afterEach(() => vi.unstubAllGlobals())

const choice: MovementChoice = { direction: 'up', target_stage: 200 }
const checkpoint: CheckpointState = {
  completed_stage_count: 1,
  uuid: 'run-1',
  pending: { stage_index: 1, direction: 'up' },
}

describe('workflow movement API', () => {
  it('submits the Application choice with its expected checkpoint', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(null, { status: 204 }))
    vi.stubGlobal('fetch', fetchMock)

    await expect(submitWorkflowMovement({ id: 'demo', choice, checkpoint })).resolves.toBeUndefined()
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/workflows/demo/movements',
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ ...choice, expected_checkpoint: checkpoint }),
      }),
    )
  })

  it('preserves a typed stale-checkpoint error from the shared transport', async () => {
    vi.stubGlobal(
      'fetch',
      vi
        .fn()
        .mockResolvedValue(
          Response.json({ error: { code: 'stale_checkpoint', message: 'state changed' } }, { status: 409 }),
        ),
    )

    const error = await submitWorkflowMovement({ id: 'demo', choice, checkpoint }).catch((value: unknown) => value)
    expect(error).toBeInstanceOf(ApiFailure)
    expect(error).toMatchObject({ code: 'stale_checkpoint', status: 409, message: 'state changed' })
  })
})
