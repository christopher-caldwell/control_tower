import type {
  CheckpointState,
  CheckpointView,
  MovementChoice,
  MovementObservation,
  StageIdentity,
  StageView,
  WorkflowIdentity,
  WorkflowView,
} from '@/api/types'

type JsonObject = Record<string, unknown>
type Direction = MovementChoice['direction']
type RoleState = MovementObservation['role_results'][number]['state']
type ObservationState = MovementObservation['state']
type StageState = StageView['state']

const isObject = (value: unknown): value is JsonObject =>
  typeof value === 'object' && value !== null && !Array.isArray(value)
const invalid = (path: string): never => {
  throw new Error('Invalid workflow snapshot at ' + path)
}

const objectAt = (value: unknown, path: string): JsonObject => (isObject(value) ? value : invalid(path))
const stringAt = (value: unknown, path: string): string => (typeof value === 'string' ? value : invalid(path))
const booleanAt = (value: unknown, path: string): boolean => (typeof value === 'boolean' ? value : invalid(path))
const numberAt = (value: unknown, path: string): number =>
  typeof value === 'number' && Number.isFinite(value) ? value : invalid(path)
const nullableStringAt = (value: unknown, path: string): string | null =>
  value === null ? null : stringAt(value, path)
const arrayAt = (value: unknown, path: string): unknown[] => (Array.isArray(value) ? value : invalid(path))
const enumAt = <Value extends string>(value: unknown, values: readonly Value[], path: string): Value =>
  typeof value === 'string' && values.includes(value as Value) ? (value as Value) : invalid(path)

const readDirection = (value: unknown, path: string): Direction => enumAt(value, ['up', 'down'], path)

const readIdentity = (value: unknown, path: string): StageIdentity => {
  const identity = objectAt(value, path)
  return {
    number: numberAt(identity.number, path + '.number'),
    name: stringAt(identity.name, path + '.name'),
  }
}

const readWorkflowIdentity = (value: unknown, path: string): WorkflowIdentity => {
  const workflow = objectAt(value, path)
  return {
    id: stringAt(workflow.id, path + '.id'),
    name: stringAt(workflow.name, path + '.name'),
  }
}

const readMovementChoice = (value: unknown, path: string): MovementChoice => {
  const choice = objectAt(value, path)
  return {
    direction: readDirection(choice.direction, path + '.direction'),
    target_stage: numberAt(choice.target_stage, path + '.target_stage'),
  }
}

const readCheckpointState = (value: unknown, path: string): CheckpointState => {
  const state = objectAt(value, path)
  let pending: CheckpointState['pending'] = null
  if (state.pending !== null) {
    const pendingState = objectAt(state.pending, path + '.pending')
    pending = {
      stage_index: numberAt(pendingState.stage_index, path + '.pending.stage_index'),
      direction: readDirection(pendingState.direction, path + '.pending.direction'),
    }
  }
  return {
    completed_stage_count: numberAt(state.completed_stage_count, path + '.completed_stage_count'),
    uuid: nullableStringAt(state.uuid, path + '.uuid'),
    pending,
  }
}

const readCheckpoint = (value: unknown): CheckpointView | null => {
  if (value === null) return null
  const checkpoint = objectAt(value, 'checkpoint')
  let acceptedStage: StageIdentity | null = null
  if (checkpoint.accepted_stage !== null)
    acceptedStage = readIdentity(checkpoint.accepted_stage, 'checkpoint.accepted_stage')

  let pendingTransition: CheckpointView['pending_transition'] = null
  if (checkpoint.pending_transition !== null) {
    const transition = objectAt(checkpoint.pending_transition, 'checkpoint.pending_transition')
    pendingTransition = {
      stage: readIdentity(transition.stage, 'checkpoint.pending_transition.stage'),
      direction: readDirection(transition.direction, 'checkpoint.pending_transition.direction'),
    }
  }
  return {
    accepted_stage: acceptedStage,
    pending_transition: pendingTransition,
    state: readCheckpointState(checkpoint.state, 'checkpoint.state'),
  }
}

const readRoleObservation = (value: unknown, path: string): MovementObservation['role_results'][number] => {
  const result = objectAt(value, path)
  const roleState: RoleState = enumAt(
    result.state,
    ['in_progress', 'succeeded', 'failed', 'launch_failed'],
    path + '.state',
  )
  let exitCode: number | null = null
  if (result.exit_code !== null) exitCode = numberAt(result.exit_code, path + '.exit_code')
  return {
    stage: readIdentity(result.stage, path + '.stage'),
    role: stringAt(result.role, path + '.role'),
    state: roleState,
    exit_code: exitCode,
    message: nullableStringAt(result.message, path + '.message'),
    stdout: nullableStringAt(result.stdout, path + '.stdout'),
    stderr: nullableStringAt(result.stderr, path + '.stderr'),
  }
}

const readFailure = (value: unknown, path: string): MovementObservation['failure'] => {
  if (value === null) return null
  const failure = objectAt(value, path)
  let stage: StageIdentity | null = null
  if (failure.stage !== null) stage = readIdentity(failure.stage, path + '.stage')
  return {
    kind: stringAt(failure.kind, path + '.kind'),
    message: stringAt(failure.message, path + '.message'),
    stage,
    role: nullableStringAt(failure.role, path + '.role'),
  }
}

const readVerificationChoices = (value: unknown): MovementObservation['verification_choices'] => {
  if (value === null) return null
  const choices = objectAt(value, 'observation.verification_choices')
  let reverse: MovementChoice | null = null
  if (choices.reverse !== null)
    reverse = readMovementChoice(choices.reverse, 'observation.verification_choices.reverse')
  return {
    retry: readMovementChoice(choices.retry, 'observation.verification_choices.retry'),
    reverse,
  }
}

const readObservation = (value: unknown): MovementObservation | null => {
  if (value === null) return null
  const observation = objectAt(value, 'observation')
  const observationState: ObservationState = enumAt(
    observation.state,
    ['running', 'complete', 'stopped'],
    'observation.state',
  )
  let activeRole: MovementObservation['active_role'] = null
  if (observation.active_role !== null) {
    const role = objectAt(observation.active_role, 'observation.active_role')
    activeRole = {
      stage: readIdentity(role.stage, 'observation.active_role.stage'),
      role: stringAt(role.role, 'observation.active_role.role'),
    }
  }
  const roleResults = arrayAt(observation.role_results, 'observation.role_results').map((result, index) =>
    readRoleObservation(result, 'observation.role_results[' + index + ']'),
  )
  return {
    direction: readDirection(observation.direction, 'observation.direction'),
    target_stage: numberAt(observation.target_stage, 'observation.target_stage'),
    state: observationState,
    active_role: activeRole,
    role_results: roleResults,
    failure: readFailure(observation.failure, 'observation.failure'),
    verification_choices: readVerificationChoices(observation.verification_choices),
  }
}

const readStage = (value: unknown, path: string): StageView => {
  const stage = objectAt(value, path)
  const stageState: StageState = enumAt(stage.state, ['accepted', 'pending', 'future'], path + '.state')
  const definitions: StageView['definitions'] = arrayAt(stage.definitions, path + '.definitions').map(
    (value, index) => {
      const definition = objectAt(value, path + '.definitions[' + index + ']')
      return {
        role: stringAt(definition.role, path + '.definitions[' + index + '].role'),
        path: stringAt(definition.path, path + '.definitions[' + index + '].path'),
      }
    },
  )
  return {
    ...readIdentity(stage, path),
    state: stageState,
    is_accepted_checkpoint: booleanAt(stage.is_accepted_checkpoint, path + '.is_accepted_checkpoint'),
    definitions,
  }
}

export const parseSnapshot = (data: string, workflowId: string): WorkflowView => {
  let parsed: unknown
  try {
    parsed = JSON.parse(data)
  } catch {
    return invalid('JSON')
  }
  const snapshot = objectAt(parsed, 'root')
  const workflow = readWorkflowIdentity(snapshot.workflow, 'workflow')
  if (workflow.id !== workflowId) return invalid('workflow.id (does not match subscription)')
  return {
    workspace_name: stringAt(snapshot.workspace_name, 'workspace_name'),
    workflow,
    current_status: enumAt(snapshot.current_status, ['available', 'unavailable'], 'current_status'),
    status_issue: nullableStringAt(snapshot.status_issue, 'status_issue'),
    checkpoint: readCheckpoint(snapshot.checkpoint),
    movement_choices: arrayAt(snapshot.movement_choices, 'movement_choices').map((choice, index) =>
      readMovementChoice(choice, 'movement_choices[' + index + ']'),
    ),
    stages: arrayAt(snapshot.stages, 'stages').map((stage, index) => readStage(stage, 'stages[' + index + ']')),
    movement_busy: booleanAt(snapshot.movement_busy, 'movement_busy'),
    observation: readObservation(snapshot.observation),
  }
}
