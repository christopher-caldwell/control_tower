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
