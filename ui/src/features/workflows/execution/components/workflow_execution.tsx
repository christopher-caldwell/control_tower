import { Stack, Text, Title } from '@mantine/core'
import type { FC } from 'react'

import type { WorkflowIdentity } from '@/api/types'
import { MovementDock } from '@/features/workflows/execution/components/movement_dock'
import styles from '@/features/workflows/execution/components/workflow_execution.module.css'
import { WorkflowStageList } from '@/features/workflows/execution/components/workflow_stage_list'
import { WorkflowSummary } from '@/features/workflows/execution/components/workflow_summary'
import type { WorkflowExecutionModel } from '@/features/workflows/execution/hooks/use_workflow_execution'

export type WorkflowExecutionProps = {
  model: WorkflowExecutionModel
  selectedWorkflow: WorkflowIdentity | null
  emptyStateMessage: string | null
  unavailableLabel: string
}

export const WorkflowExecution: FC<WorkflowExecutionProps> = ({
  model,
  selectedWorkflow,
  emptyStateMessage,
  unavailableLabel,
}) => (
  <main className={styles.center} aria-label="Ordered stages">
    <div className={styles.executionBody}>
      <WorkflowSummary model={model} selectedWorkflow={selectedWorkflow} />
      <WorkflowStageList model={model} />
      {emptyStateMessage ? <EmptyWorkflowState message={emptyStateMessage} /> : null}
    </div>
    <MovementDock model={model} unavailableLabel={unavailableLabel} />
  </main>
)

type EmptyWorkflowStateProps = { message: string }
const EmptyWorkflowState: FC<EmptyWorkflowStateProps> = ({ message }) => (
  <Stack align="center" mt="xl">
    <Title order={2}>No workflows found</Title>
    <Text c="dimmed" ta="center">
      {message}
    </Text>
  </Stack>
)
