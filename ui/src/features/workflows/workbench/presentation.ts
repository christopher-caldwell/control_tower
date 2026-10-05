import type { CheckpointView, MovementChoice, WorkflowView } from '@/api/types'
export type Action = { choice: MovementChoice; label: string; detail: string }
export const movementActions = (view: WorkflowView): Action[] => {
  const checkpoint = view.checkpoint
  if (!checkpoint) return []
  const pending = checkpoint.pending_transition,
    observation = view.observation
  const recovery = pending && observation?.state === 'stopped' ? observation.verification_choices : null
  const label = (number: number) =>
    number === 0
      ? 'baseline'
      : 'Stage ' + number + ' · ' + (view.stages.find((item) => item.number === number)?.name ?? 'unknown')
  if (recovery && pending)
    return [
      {
        choice: recovery.retry,
        label: 'Retry verify-' + recovery.retry.direction + ' for Stage ' + pending.stage.number,
        detail: 'The mutation is not repeated by this verifier retry.',
      },
      ...(recovery.reverse
        ? [
            {
              choice: recovery.reverse,
              label:
                recovery.reverse.direction === 'down'
                  ? 'Back out Stage ' + pending.stage.number + ' to ' + label(recovery.reverse.target_stage)
                  : 'Reapply Stage ' + pending.stage.number + ' upward',
              detail: 'Runs the reverse mutation and optional verifier.',
            },
          ]
        : []),
    ]
  return view.movement_choices.map((choice) => {
    if (pending)
      return {
        choice,
        label:
          choice.direction === pending.direction
            ? 'Continue pending ' + pending.direction + ' · Stage ' + pending.stage.number
            : choice.direction === 'down'
              ? 'Back out Stage ' + pending.stage.number + ' to ' + label(choice.target_stage)
              : 'Reapply Stage ' + pending.stage.number + ' upward',
        detail: 'Application supplied this immediate movement.',
      }
    const stage =
      choice.direction === 'up'
        ? view.stages.find((item) => item.number === choice.target_stage)
        : checkpoint.accepted_stage
    return {
      choice,
      label:
        choice.direction === 'up'
          ? 'Advance to ' + label(choice.target_stage)
          : 'Back out Stage ' + (stage?.number ?? 'unknown') + ' to ' + label(choice.target_stage),
      detail: 'Application supplied this immediate movement.',
    }
  })
}
export const forwardActions = (
  view: WorkflowView,
  selectedStageNumber: number | null,
): { next: Action | null; to: Action | null; all: Action | null } => {
  if (view.checkpoint?.pending_transition) {
    return { next: null, to: null, all: null }
  }
  const immediate = view.movement_choices.find((choice) => choice.direction === 'up')
  if (!immediate) return { next: null, to: null, all: null }

  const action = (target: number, label: string, detail: string): Action => ({
    choice: { ...immediate, target_stage: target },
    label,
    detail,
  })
  const next: Action = {
    choice: immediate,
    label: 'Run next',
    detail: 'Run only the next unapplied stage.',
  }
  const nextIndex = view.stages.findIndex((stage) => stage.number === immediate.target_stage)
  const selectedIndex = view.stages.findIndex((stage) => stage.number === selectedStageNumber)
  const selected = view.stages[selectedIndex]
  const to =
    selected && selected.state === 'future' && nextIndex >= 0 && selectedIndex > nextIndex
      ? action(
          selected.number,
          'Run to Stage ' + selected.number + ' · ' + selected.name,
          'Run the unapplied stages in order through the selected stage.',
        )
      : null

  const finalStage = view.stages.at(-1)
  const all =
    finalStage && finalStage.state === 'future' && finalStage.number !== immediate.target_stage
      ? action(finalStage.number, 'Run all', 'Run every remaining stage in workflow order.')
      : null
  return { next, to, all }
}
export const checkpointPosition = (checkpoint: CheckpointView | null): string => {
  return checkpoint?.accepted_stage
    ? 'Stage ' + checkpoint.accepted_stage.number + ' · ' + checkpoint.accepted_stage.name
    : 'Baseline · no accepted stages'
}
