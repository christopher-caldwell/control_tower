import type { WorkflowExecutionModel } from '@/features/workflows/execution'
import type { WorkspaceWorkflowsModel } from '@/features/workspace/workflows'

export const unavailableMovementLabel = (
  workflows: WorkspaceWorkflowsModel,
  execution: WorkflowExecutionModel,
): string => {
  if (!execution.workflowId) return 'Select a workflow'
  if (workflows.issue) return 'Workspace unavailable'
  if (!execution.workflow) return 'Connecting to workflow'
  if (execution.workflow.current_status === 'unavailable') return 'Workflow unavailable'

  const finalStage = execution.workflow.stages.at(-1)
  const acceptedStage = execution.workflow.checkpoint?.accepted_stage
  const isFinalStageAccepted = Boolean(
    finalStage && acceptedStage?.number === finalStage.number && !execution.workflow.checkpoint?.pending_transition,
  )
  if (isFinalStageAccepted) return 'All stages applied'
  return 'No immediate movement available'
}
