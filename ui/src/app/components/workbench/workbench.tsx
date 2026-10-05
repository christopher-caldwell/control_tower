import type { FC } from 'react'
import { useState } from 'react'

import { unavailableMovementLabel } from '@/app/components/workbench/unavailable_movement_label'
import styles from '@/app/components/workbench/workbench.module.css'
import { WorkbenchHeader } from '@/app/components/workbench/workbench_header'
import { useWorkflowExecution, WorkflowExecution } from '@/features/workflows/execution'
import { StageInspector, useStageInspection } from '@/features/workflows/stage_inspection'
import { useWorkspaceWorkflows, WorkflowRail } from '@/features/workspace/workflows'

export const Workbench: FC = () => {
  const workflows = useWorkspaceWorkflows()
  const execution = useWorkflowExecution({ workflowId: workflows.selectedWorkflowId })
  const inspection = useStageInspection({
    workflowId: execution.workflowId,
    stage: execution.selectedStage,
    checkpoint: execution.workflow?.checkpoint ?? null,
    observation: execution.workflow?.observation ?? null,
  })
  const [isWorkflowRailCollapsed, setWorkflowRailCollapsed] = useState(false)
  const [isInspectorCollapsed, setInspectorCollapsed] = useState(false)
  const shouldShowEmptyState =
    !workflows.isLoading && !workflows.issue && (workflows.workspace?.workflows.length ?? 0) === 0
  const emptyStateMessage = shouldShowEmptyState
    ? 'Add a workflow directory under workflows/ and restart the UI.'
    : null
  const movementLabel = unavailableMovementLabel(workflows, execution)
  const shouldDisableRefresh = workflows.selectedWorkflowId === null || execution.pendingMovement !== null

  return (
    <div className={styles.frame}>
      <WorkbenchHeader
        workspaceName={workflows.workspace?.name ?? null}
        liveLabel={execution.liveLabel}
        isConnected={execution.isConnected}
        workspaceIssue={workflows.issue}
        canRefresh={!shouldDisableRefresh}
        onRefresh={execution.refresh}
      />
      <div className={getGridClassName(isWorkflowRailCollapsed, isInspectorCollapsed)}>
        <WorkflowRail
          model={workflows}
          collapsed={isWorkflowRailCollapsed}
          onToggleCollapsed={() => setWorkflowRailCollapsed((isCollapsed) => !isCollapsed)}
        />
        <WorkflowExecution
          model={execution}
          selectedWorkflow={workflows.selectedWorkflow}
          emptyStateMessage={emptyStateMessage}
          unavailableLabel={movementLabel}
        />
        <StageInspector
          model={inspection}
          open={!isInspectorCollapsed}
          onCollapse={() => setInspectorCollapsed(true)}
          onExpand={() => setInspectorCollapsed(false)}
        />
      </div>
    </div>
  )
}

const getGridClassName = (isWorkflowRailCollapsed: boolean, isInspectorCollapsed: boolean): string => {
  const classNames = [styles.grid]
  if (isWorkflowRailCollapsed) classNames.push(styles.leftCollapsed)
  if (isInspectorCollapsed) classNames.push(styles.rightCollapsed)
  return classNames.join(' ')
}
