export type StageIdentity = { number: number; name: string };
export type PendingTransition = { direction: "up" | "down"; stage: StageIdentity };
export type WorkspaceSummary = { id: string; name: string; available: boolean; issue: string | null; stage_count: number | null; accepted_stage: StageIdentity | null; pending_transition: PendingTransition | null };
export type ProjectView = { name: string; workspace_root: string; workspaces: WorkspaceSummary[] };
export type StageView = { number: number; name: string; state: "accepted" | "pending" | "future"; is_accepted_checkpoint: boolean; definitions: { role: string; path: string }[] };
export type CheckpointState = { completed_stage_count: number; uuid: string | null; pending: { stage_index: number; direction: "up" | "down" } | null };
export type CheckpointView = { accepted_stage: StageIdentity | null; pending_transition: PendingTransition | null; workflow_started: boolean; state: CheckpointState };
export type MovementChoice = { direction: "up" | "down"; target_stage: number };
export type RoleObservation = { stage: StageIdentity; role: string; state: "in_progress" | "succeeded" | "failed" | "launch_failed"; exit_code: number | null; message: string | null; elapsed_ms: number | null; output_available: boolean; stdout_bytes: number; stderr_bytes: number };
export type MovementObservation = { operation_id: string; direction: "up" | "down"; target_stage: number; state: "running" | "complete" | "stopped"; active_role: { stage: StageIdentity; role: string } | null; role_results: RoleObservation[]; confirmed_checkpoint: CheckpointView | null; attempted_checkpoint: CheckpointView | null; failure: { kind: string; message: string; stage: StageIdentity | null; role: string | null } | null; verification_choices: { retry: MovementChoice; reverse: MovementChoice | null } | null };
export type WorkspaceView = { project_name: string; workspace: { id: string; name: string }; current_status: "available" | "unavailable"; status_issue: string | null; checkpoint: CheckpointView | null; movement_choices: MovementChoice[]; selected_stage_number: number | null; stages: StageView[]; movement_busy: boolean; observation: MovementObservation | null };
export type DefinitionView = { stage: StageIdentity; definitions: { role: string; path: string; contents: string | null; truncated: boolean; issue: string | null }[] };
export class ApiFailure extends Error { constructor(public code: string, message: string, public status: number) { super(message); } }
export async function api<T>(path: string, signal?: AbortSignal): Promise<T> {
  const response = await fetch(path, { credentials: "same-origin", signal });
  if (!response.ok) { const payload = await response.json().catch(() => null) as { error?: { code?: string; message?: string } } | null; throw new ApiFailure(payload?.error?.code ?? "request_failed", payload?.error?.message ?? `Request failed (${response.status})`, response.status); }
  return response.status === 204 ? undefined as T : await response.json() as T;
}
export async function submitMovement(path: string, movement: MovementChoice, expected_checkpoint: CheckpointState): Promise<void> {
  const response = await fetch(path, { method: "POST", credentials: "same-origin", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ ...movement, expected_checkpoint }) });
  if (!response.ok) { const payload = await response.json().catch(() => null) as { error?: { code?: string; message?: string } } | null; throw new ApiFailure(payload?.error?.code ?? "movement_failed", payload?.error?.message ?? `Movement request failed (${response.status})`, response.status); }
}
