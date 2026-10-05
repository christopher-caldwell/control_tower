import { queryOptions } from '@tanstack/react-query'

import { api, submitMovement } from '@/api/client'
import type { CheckpointState, DefinitionView, MovementChoice, WorkspaceView } from '@/api/types'
export const keys = {
  workspace: ['workspace'] as const,
  workflow: (id: string | null) => ['workflow', id] as const,
  definition: (id: string | null, stage: number | null) => ['definition', id, stage] as const,
}
export const workspaceOptions = queryOptions({
  queryKey: keys.workspace,
  queryFn: ({ signal }) => api<WorkspaceView>('/api/workspace', signal),
  staleTime: Infinity,
})
export const definitionOptions = (id: string | null, stage: number | null) =>
  queryOptions({
    queryKey: keys.definition(id, stage),
    enabled: id !== null && stage !== null,
    queryFn: ({ signal }) => api<DefinitionView>(`/api/workflows/${encodeURIComponent(id!)}/stages/${stage}`, signal),
  })
export const movementOptions = {
  retry: false as const,
  mutationFn: ({ id, choice, checkpoint }: { id: string; choice: MovementChoice; checkpoint: CheckpointState }) =>
    submitMovement(`/api/workflows/${encodeURIComponent(id)}/movements`, choice, checkpoint),
}
