import { queryOptions } from '@tanstack/react-query'

import { api } from '@/api/client'
import type { WorkspaceView } from '@/api/types'

export const workspaceOptions = queryOptions({
  queryKey: ['workspace'],
  queryFn: ({ signal }) => api<WorkspaceView>('/api/workspace', signal),
  staleTime: Infinity,
})
