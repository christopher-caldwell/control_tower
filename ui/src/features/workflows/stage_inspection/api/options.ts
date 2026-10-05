import { queryOptions } from '@tanstack/react-query'

import { api } from '@/api/client'
import type { DefinitionView } from '@/api/types'

const definitionKey = (workflowId: string | null, stageNumber: number | null) =>
  ['definition', workflowId, stageNumber] as const
export const definitionOptions = (workflowId: string | null, stageNumber: number | null) =>
  queryOptions({
    queryKey: definitionKey(workflowId, stageNumber),
    enabled: workflowId !== null && stageNumber !== null,
    queryFn: ({ signal }) =>
      api<DefinitionView>('/api/workflows/' + encodeURIComponent(workflowId!) + '/stages/' + stageNumber, signal),
  })
