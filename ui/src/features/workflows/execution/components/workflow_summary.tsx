import { Alert, Badge, Button, Group, Loader, Stack, Text, Title } from '@mantine/core'
import type { FC } from 'react'

import type { WorkflowIdentity } from '@/api/types'
import { checkpointPosition } from '@/features/workflows/execution/components/movement_actions'
import styles from '@/features/workflows/execution/components/workflow_execution.module.css'
import type { WorkflowExecutionModel } from '@/features/workflows/execution/hooks/use_workflow_execution'

type WorkflowSummaryProps = { model: WorkflowExecutionModel; selectedWorkflow: WorkflowIdentity | null }

export const WorkflowSummary: FC<WorkflowSummaryProps> = ({ model, selectedWorkflow }) => {
  const workflow = model.workflow
  const statusIssue = workflow?.status_issue
  const isWorkflowUnavailable = workflow?.current_status === 'unavailable'
  const hasStoppedFailure = Boolean(workflow?.observation?.state === 'stopped' && workflow.observation.failure)
  const stoppedFailureMessage = workflow?.observation?.failure?.message
  const shouldShowStatusIssue = Boolean(isWorkflowUnavailable && statusIssue)
  const shouldShowConnectionNotice = !model.isConnected && model.workflowId !== null
  const connectionMessage =
    model.liveLabel === 'Reconnecting'
      ? 'Connection lost. Reconnecting to the live view…'
      : 'Waiting for a fresh workflow snapshot…'
  return (
    <>
      <Group justify="space-between" align="flex-start" mb="xl">
        <Stack gap={4} style={{ flex: '1 1 180px', minWidth: 0 }}>
          <Text size="xs" c="dimmed">
            WORKFLOW / {model.selectedSummary?.id ?? '—'}
          </Text>
          <Title order={1} className={styles.longText}>
            {selectedWorkflow?.name ?? model.selectedSummary?.name ?? 'Select a workflow'}
          </Title>
          <Text size="sm" c="dimmed">
            Ordered stages and current position.
          </Text>
        </Stack>
        {workflow?.checkpoint ? <CheckpointDisplay model={model} selectedWorkflow={selectedWorkflow} /> : null}
      </Group>
      {shouldShowStatusIssue ? (
        <Alert color="red" title="Current checkpoint unavailable" mb="md">
          {statusIssue}
          <Text size="xs" mt="xs">
            Movement is disabled until current status is readable.
          </Text>
        </Alert>
      ) : null}
      {hasStoppedFailure ? (
        <Alert color="red" title="Movement stopped" mb="md">
          {stoppedFailureMessage}
        </Alert>
      ) : null}
      {shouldShowConnectionNotice ? (
        <Group mb="md" gap="xs">
          <Loader size="xs" />
          <Text size="sm" c="dimmed">
            {connectionMessage}
          </Text>
          <Button size="compact-xs" variant="subtle" onClick={model.refresh}>
            Refresh workflow
          </Button>
        </Group>
      ) : null}
    </>
  )
}

const CheckpointDisplay: FC<WorkflowSummaryProps> = ({ model }) => {
  const checkpoint = model.workflow?.checkpoint
  if (!checkpoint) return null
  const pendingTransition = checkpoint.pending_transition
  const activeRole = model.workflow?.observation?.active_role
  return (
    <Stack gap={6} align="flex-end" style={{ maxWidth: '100%' }}>
      <Text size="xs" c="dimmed">
        CURRENT CHECKPOINT
      </Text>
      <Text size="sm" fw={600} className={styles.longText}>
        {checkpointPosition(checkpoint)}
      </Text>
      {pendingTransition ? (
        <Badge color="yellow">
          Pending {pendingTransition.direction} · Stage {pendingTransition.stage.number}
        </Badge>
      ) : null}
      {model.workflow?.movement_busy ? (
        <Badge color="teal">Movement active{activeRole ? ' · ' + activeRole.role : ''}</Badge>
      ) : null}
    </Stack>
  )
}
