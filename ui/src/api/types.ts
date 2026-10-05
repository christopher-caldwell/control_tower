export type StageIdentity = { number: number; name: string }
export type PendingTransition = { direction: 'up' | 'down'; stage: StageIdentity }
export type WorkflowIdentity = { id: string; name: string }
export type WorkspaceView = { name: string; workflows: WorkflowIdentity[] }
export type StageView = {
  number: number
  name: string
  state: 'accepted' | 'pending' | 'future'
  is_accepted_checkpoint: boolean
  definitions: { role: string; path: string }[]
}
export type CheckpointState = {
  completed_stage_count: number
  uuid: string | null
  pending: { stage_index: number; direction: 'up' | 'down' } | null
}
export type CheckpointView = {
  accepted_stage: StageIdentity | null
  pending_transition: PendingTransition | null
  state: CheckpointState
}
export type MovementChoice = { direction: 'up' | 'down'; target_stage: number }
export type RoleObservation = {
  stage: StageIdentity
  role: string
  state: 'in_progress' | 'succeeded' | 'failed' | 'launch_failed'
  exit_code: number | null
  message: string | null
  stdout: string | null
  stderr: string | null
}
export type MovementObservation = {
  direction: 'up' | 'down'
  target_stage: number
  state: 'running' | 'complete' | 'stopped'
  active_role: { stage: StageIdentity; role: string } | null
  role_results: RoleObservation[]
  failure: { kind: string; message: string; stage: StageIdentity | null; role: string | null } | null
  verification_choices: { retry: MovementChoice; reverse: MovementChoice | null } | null
}
export type WorkflowView = {
  workspace_name: string
  workflow: WorkflowIdentity
  current_status: 'available' | 'unavailable'
  status_issue: string | null
  checkpoint: CheckpointView | null
  movement_choices: MovementChoice[]
  stages: StageView[]
  movement_busy: boolean
  observation: MovementObservation | null
}
export type DefinitionView = {
  stage: StageIdentity
  definitions: { role: string; path: string; contents: string | null; issue: string | null }[]
}
