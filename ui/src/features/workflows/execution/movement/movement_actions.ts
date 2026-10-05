import type { CheckpointView, MovementChoice, WorkflowView } from '@/api/types'

export type Action = { choice: MovementChoice; label: string; detail: string }
export type ForwardActions = { next: Action | null; to: Action | null; all: Action | null }

const stageLabel = (workflow: WorkflowView, number: number): string => {
  if (number === 0) return 'baseline'
  const stage = workflow.stages.find((item) => item.number === number)
  return 'Stage ' + number + ' · ' + (stage?.name ?? 'unknown')
}

const recoveryActions = (
  workflow: WorkflowView,
  pendingStage: number,
  choices: NonNullable<NonNullable<WorkflowView['observation']>['verification_choices']>,
): Action[] => {
  const actions: Action[] = [
    {
      choice: choices.retry,
      label: 'Retry verify-' + choices.retry.direction + ' for Stage ' + pendingStage,
      detail: 'The mutation is not repeated by this verifier retry.',
    },
  ]
  if (!choices.reverse) return actions
  const reverseLabel =
    choices.reverse.direction === 'down'
      ? 'Back out Stage ' + pendingStage + ' to ' + stageLabel(workflow, choices.reverse.target_stage)
      : 'Reapply Stage ' + pendingStage + ' upward'
  actions.push({
    choice: choices.reverse,
    label: reverseLabel,
    detail: 'Runs the reverse mutation and optional verifier.',
  })
  return actions
}

const pendingMovementLabel = (
  choice: MovementChoice,
  pending: NonNullable<CheckpointView['pending_transition']>,
  workflow: WorkflowView,
): string => {
  if (choice.direction === pending.direction)
    return 'Continue pending ' + pending.direction + ' · Stage ' + pending.stage.number
  if (choice.direction === 'down')
    return 'Back out Stage ' + pending.stage.number + ' to ' + stageLabel(workflow, choice.target_stage)
  return 'Reapply Stage ' + pending.stage.number + ' upward'
}

export const movementActions = (workflow: WorkflowView): Action[] => {
  const checkpoint = workflow.checkpoint
  if (!checkpoint) return []

  const pending = checkpoint.pending_transition
  const observation = workflow.observation
  const hasStoppedVerification = Boolean(pending && observation?.state === 'stopped')
  const recoveryChoices = hasStoppedVerification ? observation?.verification_choices : null
  if (pending && recoveryChoices) return recoveryActions(workflow, pending.stage.number, recoveryChoices)

  return workflow.movement_choices.map((choice) => {
    if (pending) {
      return {
        choice,
        label: pendingMovementLabel(choice, pending, workflow),
        detail: 'Application supplied this immediate movement.',
      }
    }
    const targetLabel =
      choice.direction === 'up'
        ? 'Advance to ' + stageLabel(workflow, choice.target_stage)
        : 'Back out Stage ' +
          (checkpoint.accepted_stage?.number ?? 'unknown') +
          ' to ' +
          stageLabel(workflow, choice.target_stage)
    return { choice, label: targetLabel, detail: 'Application supplied this immediate movement.' }
  })
}

const noForwardActions = (): ForwardActions => ({ next: null, to: null, all: null })

export const forwardActions = (workflow: WorkflowView, selectedStageNumber: number | null): ForwardActions => {
  if (workflow.checkpoint?.pending_transition) return noForwardActions()
  const immediate = workflow.movement_choices.find((choice) => choice.direction === 'up')
  if (!immediate) return noForwardActions()

  const makeAction = (target: number, label: string, detail: string): Action => ({
    choice: { ...immediate, target_stage: target },
    label,
    detail,
  })
  const next: Action = { choice: immediate, label: 'Run next', detail: 'Run only the next unapplied stage.' }
  const nextIndex = workflow.stages.findIndex((stage) => stage.number === immediate.target_stage)
  const selectedIndex = workflow.stages.findIndex((stage) => stage.number === selectedStageNumber)
  const selected = workflow.stages[selectedIndex]
  let runTo: Action | null = null
  const canRunToSelectedStage = selected?.state === 'future' && nextIndex >= 0 && selectedIndex > nextIndex
  if (canRunToSelectedStage) {
    runTo = makeAction(
      selected.number,
      'Run to Stage ' + selected.number + ' · ' + selected.name,
      'Run the unapplied stages in order through the selected stage.',
    )
  }

  const finalStage = workflow.stages.at(-1)
  let runAll: Action | null = null
  const canRunAll = finalStage?.state === 'future' && finalStage.number !== immediate.target_stage
  if (canRunAll && finalStage) {
    runAll = makeAction(finalStage.number, 'Run all', 'Run every remaining stage in workflow order.')
  }
  return { next, to: runTo, all: runAll }
}
