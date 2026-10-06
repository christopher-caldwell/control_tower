import type { ButtonProps } from '@mantine/core'
import { Button, Group, Stack, Text, Tooltip } from '@mantine/core'
import { IconArrowBackUp, IconArrowRight } from '@tabler/icons-react'
import type { FC } from 'react'

import { MovementFeedback } from '@/features/workflows/execution/components/movement_feedback'
import styles from '@/features/workflows/execution/components/workflow_execution.module.css'
import type { WorkflowExecutionModel } from '@/features/workflows/execution/hooks/use_workflow_execution'
import type { Action } from '@/features/workflows/execution/movement/movement_actions'

type MovementDockProps = { model: WorkflowExecutionModel; unavailableLabel: string }

export const MovementDock: FC<MovementDockProps> = ({ model, unavailableLabel }) => {
  const primary = model.forward.next ?? model.actions[0]
  const alternate = model.forward.next
    ? model.actions.find((action) => action.choice.direction === 'down')
    : model.actions[1]
  const defaultPrimaryAction = model.forward.next ? undefined : primary
  const hasPrimaryAction = primary !== undefined
  return (
    <Stack className={styles.dock} gap="sm">
      <MovementFeedback issue={model.movementIssue} />
      <Group justify="flex-end" gap="sm">
        {alternate ? <MovementButton action={alternate} model={model} isSecondary /> : null}
        {model.forward.next ? <MovementButton action={model.forward.next} model={model} /> : null}
        {model.forward.to ? <MovementButton action={model.forward.to} model={model} /> : null}
        {model.forward.all ? <MovementButton action={model.forward.all} model={model} /> : null}
        {defaultPrimaryAction ? <MovementButton action={defaultPrimaryAction} model={model} /> : null}
        {hasPrimaryAction ? null : <Text c="dimmed">{unavailableLabel}</Text>}
      </Group>
    </Stack>
  )
}

type MovementButtonProps = { action: Action; model: WorkflowExecutionModel; isSecondary?: boolean }
const MovementButton: FC<MovementButtonProps> = ({ action, model, isSecondary = false }) => {
  const isPrimaryAction = action === model.forward.next
  let variant: ButtonProps['variant'] = 'filled'
  if (isSecondary) variant = 'default'
  else if (isPrimaryAction) variant = 'filled'
  else if (action === model.forward.to || action === model.forward.all) variant = 'light'
  const isLoading = model.pendingMovement?.label === action.label
  const canMove = model.canSubmitMovement && model.workflow?.checkpoint !== null
  return (
    <Tooltip label={action.label} disabled={action.label.length <= 60} multiline maw={360}>
      <Button
        size="sm"
        maw="100%"
        variant={variant}
        loading={isLoading}
        disabled={!canMove}
        onClick={() => void model.run(action)}
        rightSection={isSecondary ? <IconArrowBackUp size={16} /> : <IconArrowRight size={16} />}
      >
        {action.label}
      </Button>
    </Tooltip>
  )
}
