import type { WorkflowView } from '@/api/types'

const object = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null
const nullableString = (value: unknown) => value === null || typeof value === 'string'
const number = (value: unknown) => typeof value === 'number' && Number.isFinite(value)
const identity = (value: unknown) => object(value) && number(value.number) && typeof value.name === 'string'
const choice = (value: unknown) =>
  object(value) && ['up', 'down'].includes(String(value.direction)) && number(value.target_stage)
const checkpoint = (value: unknown) =>
  value === null ||
  (object(value) &&
    (value.accepted_stage === null || identity(value.accepted_stage)) &&
    (value.pending_transition === null ||
      (object(value.pending_transition) &&
        identity(value.pending_transition.stage) &&
        ['up', 'down'].includes(String(value.pending_transition.direction)))) &&
    object(value.state) &&
    number(value.state.completed_stage_count) &&
    nullableString(value.state.uuid) &&
    (value.state.pending === null ||
      (object(value.state.pending) &&
        number(value.state.pending.stage_index) &&
        ['up', 'down'].includes(String(value.state.pending.direction)))))
const observation = (value: unknown) =>
  value === null ||
  (object(value) &&
    ['up', 'down'].includes(String(value.direction)) &&
    number(value.target_stage) &&
    ['running', 'complete', 'stopped'].includes(String(value.state)) &&
    (value.active_role === null ||
      (object(value.active_role) && identity(value.active_role.stage) && typeof value.active_role.role === 'string')) &&
    Array.isArray(value.role_results) &&
    value.role_results.every(
      (result) =>
        object(result) &&
        identity(result.stage) &&
        typeof result.role === 'string' &&
        ['in_progress', 'succeeded', 'failed', 'launch_failed'].includes(String(result.state)) &&
        (result.exit_code === null || number(result.exit_code)) &&
        nullableString(result.message) &&
        nullableString(result.stdout) &&
        nullableString(result.stderr),
    ) &&
    (value.failure === null ||
      (object(value.failure) &&
        typeof value.failure.kind === 'string' &&
        typeof value.failure.message === 'string' &&
        (value.failure.stage === null || identity(value.failure.stage)) &&
        nullableString(value.failure.role))) &&
    (value.verification_choices === null ||
      (object(value.verification_choices) &&
        choice(value.verification_choices.retry) &&
        (value.verification_choices.reverse === null || choice(value.verification_choices.reverse)))))

export const parseSnapshot = (data: string, workflowId: string): WorkflowView => {
  const value: unknown = JSON.parse(data)
  if (
    !object(value) ||
    !object(value.workflow) ||
    value.workflow.id !== workflowId ||
    typeof value.workflow.name !== 'string' ||
    typeof value.workspace_name !== 'string' ||
    !['available', 'unavailable'].includes(String(value.current_status)) ||
    !nullableString(value.status_issue) ||
    typeof value.movement_busy !== 'boolean' ||
    !checkpoint(value.checkpoint) ||
    !observation(value.observation) ||
    !Array.isArray(value.movement_choices) ||
    !value.movement_choices.every(choice) ||
    !Array.isArray(value.stages) ||
    !value.stages.every(
      (stage) =>
        identity(stage) &&
        object(stage) &&
        ['accepted', 'pending', 'future'].includes(String(stage.state)) &&
        typeof stage.is_accepted_checkpoint === 'boolean' &&
        Array.isArray(stage.definitions) &&
        stage.definitions.every(
          (definition) =>
            object(definition) && typeof definition.role === 'string' && typeof definition.path === 'string',
        ),
    )
  ) {
    throw new Error('Invalid workflow snapshot')
  }
  return value as WorkflowView
}
