import { Badge, Group, Stack, Text, Title, UnstyledButton } from '@mantine/core'
import { IconChevronRight } from '@tabler/icons-react'
import type { FC } from 'react'

import type { StageView } from '@/api/types'
import styles from '@/features/workflows/execution/components/workflow_execution.module.css'
import type { WorkflowExecutionModel } from '@/features/workflows/execution/hooks/use_workflow_execution'

type WorkflowStageListProps = { model: WorkflowExecutionModel }

export const WorkflowStageList: FC<WorkflowStageListProps> = ({ model }) => {
  const workflow = model.workflow
  if (!workflow?.checkpoint) return null
  const countLabel = workflow.stages.length === 1 ? 'stage' : 'stages'
  return (
    <section className={styles.stageSection} aria-label="Workflow stages">
      <Group justify="space-between" mb="sm">
        <Title order={2}>Stages</Title>
        <Text size="xs" c="dimmed">
          {workflow.stages.length} {countLabel}
        </Text>
      </Group>
      <div className={styles.stageScroll}>
        {workflow.stages.map((stage) => (
          <WorkflowStageItem
            key={stage.number}
            stage={stage}
            isSelected={stage.number === model.selectedStageNumber}
            workflow={workflow}
            onSelect={() => model.chooseStage(stage.number)}
          />
        ))}
      </div>
    </section>
  )
}

type WorkflowStageItemProps = {
  stage: StageView
  workflow: NonNullable<WorkflowExecutionModel['workflow']>
  isSelected: boolean
  onSelect: () => void
}
const WorkflowStageItem: FC<WorkflowStageItemProps> = ({ stage, workflow, isSelected, onSelect }) => {
  const isActiveStage = workflow.observation?.active_role?.stage.number === stage.number
  const activeRole = isActiveStage ? (workflow.observation?.active_role?.role ?? null) : null
  const failure = workflow.observation?.failure
  const failedRoles = new Set(
    workflow.observation?.role_results
      .filter(
        (result) =>
          result.stage.number === stage.number && (result.state === 'failed' || result.state === 'launch_failed'),
      )
      .map((result) => result.role),
  )
  if (failure?.stage?.number === stage.number && failure.role) failedRoles.add(failure.role)
  let stateColor = 'dimmed'
  if (stage.state === 'accepted') stateColor = 'green.4'
  if (stage.state === 'pending') stateColor = 'yellow.3'
  const stateLabel = getStageStateLabel(stage)
  return (
    <UnstyledButton className={styles.stage} data-selected={isSelected} aria-pressed={isSelected} onClick={onSelect}>
      <Group wrap="nowrap" align="center">
        <Text size="sm" fw={700} className={styles.stageNumber}>
          {stage.number}
        </Text>
        <Stack gap={4} className={styles.stageCopy}>
          <Group gap={6}>
            <Text size="xs" c={stateColor}>
              {stateLabel}
            </Text>
            {stage.is_accepted_checkpoint ? (
              <Badge color="green" size="xs">
                Checkpoint
              </Badge>
            ) : null}
            {Array.from(failedRoles).map((role) => (
              <Badge key={role} color="red" size="xs">
                {role} – failed
              </Badge>
            ))}
            {activeRole ? (
              <Badge color="teal" size="xs">
                Running {activeRole}
              </Badge>
            ) : null}
          </Group>
          <Text fw={600}>{stage.name}</Text>
          <Text size="xs" c="dimmed">
            {stage.definitions.map((role) => role.role).join(' · ')}
          </Text>
        </Stack>
        <IconChevronRight size={18} />
      </Group>
    </UnstyledButton>
  )
}

const getStageStateLabel = (stage: StageView): string => {
  if (stage.state === 'accepted') return 'Applied'
  if (stage.state === 'pending') return 'Pending transition'
  return 'Not applied'
}
