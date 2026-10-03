export type StageIdentity = { number: number; name: string };

export type PendingTransition = {
  direction: "up" | "down";
  stage: StageIdentity;
};

export type WorkspaceSummary = {
  id: string;
  name: string;
  available: boolean;
  issue: string | null;
  stage_count: number | null;
  accepted_stage: StageIdentity | null;
  pending_transition: PendingTransition | null;
};

export type ProjectView = {
  name: string;
  workspace_root: string;
  workspaces: WorkspaceSummary[];
  discovery_error: string | null;
};

export type StageView = {
  number: number;
  name: string;
  state: "accepted" | "pending" | "future";
  is_accepted_checkpoint: boolean;
  definitions: { role: string; path: string }[];
};

export type WorkspaceView = {
  project_name: string;
  workspace: { id: string; name: string };
  checkpoint: {
    accepted_stage: StageIdentity | null;
    pending_transition: PendingTransition | null;
    workflow_started: boolean;
  };
  selected_stage_number: number | null;
  stages: StageView[];
  server_instance_id: string;
  observation_revision: number;
  movement_busy: boolean;
  observation: MovementObservation | null;
  storage_issue: string | null;
};

export type CheckpointView = {
  accepted_stage: StageIdentity | null;
  pending_transition: PendingTransition | null;
  workflow_started: boolean;
};

export type MovementChoice = { direction: "up" | "down"; target_stage: number };

export type RoleObservation = {
  stage: StageIdentity;
  role: string;
  state: "in_progress" | "succeeded" | "failed" | "launch_failed";
  exit_code: number | null;
  message: string | null;
  elapsed_ms: number | null;
  output_id: string | null;
  output_state: "not_returned" | "available" | "evicted" | "unavailable";
  stdout_bytes: number;
  stderr_bytes: number;
  stdout_truncated: boolean;
  stderr_truncated: boolean;
};

export type MovementObservation = {
  workspace_id: string;
  operation_id: string;
  server_instance_id: string;
  revision: number;
  direction: "up" | "down";
  target_stage: number;
  state: "running" | "complete" | "stopped" | "unavailable";
  active_role: { stage: StageIdentity; role: string } | null;
  role_results: RoleObservation[];
  omitted_role_results: number;
  outputs_evicted: number;
  confirmed_checkpoint: CheckpointView | null;
  attempted_checkpoint: CheckpointView | null;
  failure: {
    kind: string;
    message: string;
    stage: StageIdentity | null;
    role: string | null;
  } | null;
  verification_choices: {
    retry: MovementChoice;
    reverse: MovementChoice | null;
  } | null;
};

export type RuntimeSnapshot = {
  workspace_id: string;
  server_instance_id: string;
  revision: number;
  movement_busy: boolean;
  checkpoint: CheckpointView | null;
  observation: MovementObservation | null;
};

export type DefinitionView = {
  stage: StageIdentity;
  definitions: {
    role: string;
    path: string;
    contents: string | null;
    truncated: boolean;
    issue: string | null;
  }[];
};

export async function api<T>(path: string, signal?: AbortSignal): Promise<T> {
  const response = await fetch(path, { credentials: "same-origin", signal });
  if (!response.ok) {
    const payload = (await response.json().catch(() => null)) as
      | { error?: { message?: string } }
      | null;
    throw new Error(payload?.error?.message ?? `Request failed (${response.status})`);
  }
  return (await response.json()) as T;
}

export async function submitMovement(
  path: string,
  movement: MovementChoice,
): Promise<MovementObservation> {
  const response = await fetch(path, {
    method: "POST",
    credentials: "same-origin",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(movement),
  });
  const payload = await response.json().catch(() => null) as
    | { observation?: MovementObservation; error?: { message?: string }; operation_id?: string }
    | null;
  if (!response.ok || !payload?.observation) {
    throw new Error(payload?.error?.message ?? `Movement request failed (${response.status})`);
  }
  return payload.observation;
}
