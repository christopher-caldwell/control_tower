import { useQuery } from '@tanstack/react-query'
import { useState } from 'react'

import type { WorkflowIdentity, WorkspaceView } from '@/api/types'
import { workspaceOptions } from '@/features/workspace/workflows/api/options'

const errorMessage = (error: unknown): string => (error instanceof Error ? error.message : 'Workspace request failed')

export type WorkspaceWorkflowsModel = {
  workspace: WorkspaceView | undefined
  isLoading: boolean
  issue: string | null
  selectedWorkflowId: string | null
  selectedWorkflow: WorkflowIdentity | null
  selectWorkflow: (workflowId: string) => void
  retry: () => void
}

export const useWorkspaceWorkflows = (): WorkspaceWorkflowsModel => {
  const workspaceQuery = useQuery(workspaceOptions)
  const [requestedWorkflowId, setRequestedWorkflowId] = useState<string | null>(null)
  const requestedWorkflowExists =
    workspaceQuery.data?.workflows.some((workflow) => workflow.id === requestedWorkflowId) ?? false
  const selectedWorkflowId = requestedWorkflowExists
    ? requestedWorkflowId
    : (workspaceQuery.data?.workflows[0]?.id ?? null)
  const selectedWorkflow = workspaceQuery.data?.workflows.find((workflow) => workflow.id === selectedWorkflowId) ?? null

  const selectWorkflow = (workflowId: string) => {
    const workflowExists = workspaceQuery.data?.workflows.some((workflow) => workflow.id === workflowId) ?? false
    if (workflowExists) setRequestedWorkflowId(workflowId)
  }

  return {
    workspace: workspaceQuery.data,
    isLoading: workspaceQuery.isPending,
    issue: workspaceQuery.error ? errorMessage(workspaceQuery.error) : null,
    selectedWorkflowId,
    selectedWorkflow,
    selectWorkflow,
    retry: () => void workspaceQuery.refetch(),
  }
}
