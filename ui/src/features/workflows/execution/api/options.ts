import { queryOptions, skipToken } from '@tanstack/react-query'

import { apiRequest } from '@/api/client'
import type { CheckpointState, MovementChoice, WorkflowView } from '@/api/types'

export const workflowKey = (id: string | null) => ['workflow', id] as const

export const workflowSnapshotOptions = (id: string | null) =>
  queryOptions<WorkflowView>({
    queryKey: workflowKey(id),
    queryFn: skipToken,
  })

export type MovementRequest = { id: string; choice: MovementChoice; checkpoint: CheckpointState }

export const submitWorkflowMovement = async ({ id, choice, checkpoint }: MovementRequest): Promise<void> => {
  await apiRequest('/api/workflows/' + encodeURIComponent(id) + '/movements', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ ...choice, expected_checkpoint: checkpoint }),
  })
}
