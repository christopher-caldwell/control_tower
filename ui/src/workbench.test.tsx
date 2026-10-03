import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./workbench";
import type { DefinitionView, MovementObservation, ProjectView, RoleObservation, RuntimeSnapshot, WorkspaceView } from "./types";

const project: ProjectView = {
  name: "example-project",
  workspace_root: "workspaces/",
  discovery_error: null,
  workspaces: [
    {
      id: "fixture",
      name: "fixture",
      available: true,
      issue: null,
      stage_count: 2,
      accepted_stage: { number: 10, name: "seed" },
      pending_transition: { direction: "down", stage: { number: 200, name: "finish" } },
    },
    {
      id: "unprepared",
      name: "unprepared",
      available: false,
      issue: "SQLite state is not initialized.",
      stage_count: null,
      accepted_stage: null,
      pending_transition: null,
    },
  ],
};

const workspace: WorkspaceView = {
  project_name: "example-project",
  workspace: { id: "fixture", name: "fixture" },
  server_instance_id: "server-one",
  observation_revision: 0,
  movement_busy: false,
  observation: null,
  storage_issue: null,
  checkpoint: {
    accepted_stage: { number: 200, name: "finish" },
    pending_transition: { direction: "down", stage: { number: 200, name: "finish" } },
    workflow_started: true,
  },
  selected_stage_number: 200,
  stages: [
    {
      number: 10,
      name: "seed",
      state: "accepted",
      is_accepted_checkpoint: false,
      definitions: [{ role: "up", path: "stages/010-seed/up" }, { role: "down", path: "stages/010-seed/down" }],
    },
    {
      number: 200,
      name: "finish",
      state: "pending",
      is_accepted_checkpoint: true,
      definitions: [
        { role: "up", path: "stages/200-finish/up" },
        { role: "verify-up", path: "stages/200-finish/verify-up" },
        { role: "down", path: "stages/200-finish/down" },
      ],
    },
  ],
};

const definition: DefinitionView = {
  stage: { number: 200, name: "finish" },
  definitions: [
    {
      role: "up",
      path: "stages/200-finish/up",
      contents: "#!/bin/sh\nprintf '<script>alert(1)</script>'",
      truncated: false,
      issue: null,
    },
    {
      role: "verify-up",
      path: "stages/200-finish/verify-up",
      contents: "#!/bin/sh\ntest -f result.txt",
      truncated: false,
      issue: null,
    },
    { role: "down", path: "stages/200-finish/down", contents: "#!/bin/sh\nrm -f result.txt", truncated: false, issue: null },
  ],
};

let requests: { path: string; method: string }[];
let mockFetch: ReturnType<typeof vi.fn>;
let activeProject: ProjectView;
let activeWorkspace: WorkspaceView;
let extraWorkspace: WorkspaceView | null;
let movementResponse: MovementObservation | null;
let outputResponse: ((path: string) => Promise<Response>) | null;
type FetchImplementation = (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>;

class FakeEventSource extends EventTarget {
  static instances: FakeEventSource[] = [];
  onopen: ((event: Event) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  readonly url: string;

  constructor(url: string) {
    super();
    this.url = url;
    FakeEventSource.instances.push(this);
    queueMicrotask(() => this.onopen?.(new Event("open")));
  }

  close() {}

  emit(name: string, value: RuntimeSnapshot | Record<string, unknown>) {
    this.dispatchEvent(new MessageEvent(name, { data: JSON.stringify(value) }));
  }
}

function jsonResponse(value: unknown) {
  return new Response(JSON.stringify(value), { status: 200, headers: { "Content-Type": "application/json" } });
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((complete) => { resolve = complete; });
  return { promise, resolve };
}

function roleResult(stage: number, role: string, outputId: string, state: RoleObservation["state"] = "succeeded"): RoleObservation {
  return {
    stage: { number: stage, name: stage === 10 ? "seed" : "finish" },
    role,
    state,
    exit_code: state === "succeeded" ? 0 : 9,
    message: null,
    elapsed_ms: 3,
    output_id: outputId,
    output_state: "available",
    stdout_bytes: 12,
    stderr_bytes: 0,
    stdout_truncated: false,
    stderr_truncated: false,
  };
}

beforeEach(() => {
  requests = [];
  activeProject = project;
  activeWorkspace = workspace;
  extraWorkspace = null;
  movementResponse = null;
  outputResponse = null;
  FakeEventSource.instances = [];
  localStorage.clear();
  history.replaceState(null, "", "/");
  mockFetch = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const path = String(input);
    const method = init?.method ?? "GET";
    requests.push({ path, method });
    if (path === "/api/session") return new Response(null, { status: 204 });
    if (path === "/api/project") return jsonResponse(activeProject);
    if (path === "/api/workspaces/fixture") return jsonResponse(activeWorkspace);
    if (path === "/api/workspaces/sibling" && extraWorkspace) return jsonResponse(extraWorkspace);
    if (path === "/api/workspaces/fixture/stages/200") return jsonResponse(definition);
    if (path === "/api/workspaces/fixture/stages/10") return jsonResponse({
      stage: { number: 10, name: "seed" },
      definitions: [
        { role: "up", path: "stages/010-seed/up", contents: "#!/bin/sh", truncated: false, issue: null },
        { role: "down", path: "stages/010-seed/down", contents: "#!/bin/sh", truncated: false, issue: null },
      ],
    });
    if (path === "/api/workspaces/unprepared") {
      return new Response(JSON.stringify({ error: { message: "SQLite state is not initialized." } }), { status: 503 });
    }
    if (path === "/api/workspaces/fixture/movements" && method === "POST") {
      return jsonResponse({ observation: movementResponse });
    }
    if (outputResponse && path.includes("/outputs/")) return outputResponse(path);
    if (path.includes("/outputs/") && path.endsWith("/stdout")) {
      return new Response(new Uint8Array([255, 0, 60, 115, 99, 114, 105, 112, 116, 62]), { status: 200 });
    }
    if (path.includes("/outputs/") && path.endsWith("/stderr")) {
      return new Response(new Uint8Array([101, 114, 114, 10]), { status: 200 });
    }
    return new Response("not found", { status: 404 });
  });
  vi.stubGlobal("fetch", mockFetch);
  vi.stubGlobal("EventSource", FakeEventSource as unknown as typeof EventSource);
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("desktop workbench", () => {
  it("keeps backout available while clearly identifying the final accepted position", async () => {
    activeWorkspace = {
      ...workspace,
      checkpoint: { accepted_stage: { number: 200, name: "finish" }, pending_transition: null, workflow_started: true },
      observation: null,
      stages: workspace.stages.map((stage) => ({ ...stage, state: "accepted", is_accepted_checkpoint: stage.number === 200 })),
    };
    render(<App />);

    expect(await screen.findByText("All stages applied; backout remains available")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Back out Stage 200 to Stage 10 · seed/ })).toBeEnabled();
  });

  it("initial selection follows the accepted checkpoint over a retained down result", async () => {
    const accepted = { accepted_stage: { number: 10, name: "seed" }, pending_transition: null, workflow_started: true };
    const downResult: MovementObservation = {
      workspace_id: "fixture", operation_id: "retained-down-result", server_instance_id: "server-one", revision: 7,
      direction: "down", target_stage: 10, state: "complete", active_role: null,
      role_results: [roleResult(200, "down", "retained-stage-200-down")], omitted_role_results: 0, outputs_evicted: 0,
      confirmed_checkpoint: accepted, attempted_checkpoint: null, failure: null, verification_choices: null,
    };
    activeWorkspace = {
      ...workspace,
      checkpoint: accepted,
      observation_revision: 7,
      observation: downResult,
      selected_stage_number: 200,
      stages: workspace.stages.map((stage) => ({
        ...stage,
        state: stage.number === 10 ? "accepted" : "future",
        is_accepted_checkpoint: stage.number === 10,
      })),
    };
    activeProject = {
      ...project,
      workspaces: project.workspaces.map((item) => item.id === "fixture"
        ? { ...item, accepted_stage: accepted.accepted_stage, pending_transition: null }
        : item),
    };

    render(<App />);

    const acceptedStage = await screen.findByRole("button", { name: /STAGE 010/ });
    expect(acceptedStage).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: /STAGE 200/ })).toHaveAttribute("aria-pressed", "false");
    expect(screen.getByRole("heading", { name: "seed" })).toBeInTheDocument();
    expect(screen.getByText("10 · seed")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Back out Stage 10 to baseline/ })).toBeEnabled();
  });

  it("keeps a locally initiated down movement focused on its result while fresh selection follows checkpoint", async () => {
    const user = userEvent.setup();
    const startCheckpoint = { accepted_stage: { number: 200, name: "finish" }, pending_transition: null, workflow_started: true };
    const accepted = { accepted_stage: { number: 10, name: "seed" }, pending_transition: null, workflow_started: true };
    activeWorkspace = {
      ...workspace,
      checkpoint: startCheckpoint,
      observation: null,
      selected_stage_number: 200,
      stages: workspace.stages.map((stage) => ({ ...stage, state: "accepted", is_accepted_checkpoint: stage.number === 200 })),
    };
    movementResponse = {
      workspace_id: "fixture", operation_id: "local-down-result", server_instance_id: "server-one", revision: 8,
      direction: "down", target_stage: 10, state: "complete", active_role: null,
      role_results: [roleResult(200, "down", "local-stage-200-down")], omitted_role_results: 0, outputs_evicted: 0,
      confirmed_checkpoint: accepted, attempted_checkpoint: null, failure: null, verification_choices: null,
    };
    render(<App />);

    await user.click(await screen.findByRole("button", { name: /Back out Stage 200 to Stage 10 · seed/ }));

    expect(await screen.findAllByText("Process succeeded · 3 ms")).toHaveLength(2);
    expect(screen.getByRole("heading", { name: "finish" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /STAGE 200/ })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByText("10 · seed")).toBeInTheDocument();
  });

  it("labels an empty project without implying that its stages are applied", async () => {
    activeProject = { ...project, workspaces: [] };
    render(<App />);

    expect(await screen.findByText("No workspaces found")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Select a workspace/ })).toBeDisabled();
    expect(screen.queryByText("All stages applied")).not.toBeInTheDocument();
  });

  it("keeps pending state distinct from the accepted checkpoint and renders definitions inertly", async () => {
    render(<App />);
    expect(await screen.findByRole("heading", { name: "finish" })).toBeInTheDocument();
    expect(screen.getAllByText("Pending down").length).toBeGreaterThanOrEqual(2);
    expect(screen.getByText("This is also the last confirmed checkpoint. The pending transition has not been accepted.")).toBeInTheDocument();
    expect(screen.getByText("Pending · prior outcome unavailable")).toBeInTheDocument();
    expect(screen.getAllByText("Unknown").length).toBeGreaterThanOrEqual(3);
    expect(screen.getAllByText("Prior outcome unavailable").length).toBeGreaterThanOrEqual(2);
    expect(screen.queryByText("Not run")).not.toBeInTheDocument();

    const source = await screen.findByText(/<script>alert\(1\)<\/script>/);
    expect(source.tagName.toLowerCase()).toBe("pre");
    expect(document.querySelector("pre script")).toBeNull();
    expect(screen.getByRole("button", { name: /Continue pending down · Stage 200/ })).toBeEnabled();
    expect(screen.getByRole("button", { name: /Reapply Stage 200 upward/ })).toBeEnabled();
    expect(requests.every((request) => request.method === "GET")).toBe(true);
  });

  it("allows stage inspection and manual rail collapse without retargeting movement controls", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByRole("heading", { name: "finish" });

    await user.click(screen.getByRole("button", { name: "Collapse stage inspector" }));
    expect(screen.getByRole("button", { name: "Expand stage inspector" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /STAGE 010/ }));
    expect(screen.getByRole("button", { name: "Expand stage inspector" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Continue pending down · Stage 200/ })).toBeEnabled();

    await user.click(screen.getByRole("button", { name: "Expand stage inspector" }));
    expect(await screen.findByText("Accepted earlier · execution evidence not retained")).toBeInTheDocument();
    expect(screen.getAllByText("Applied · historical result unavailable")).toHaveLength(2);
    expect(screen.getByRole("button", { name: /Continue pending down · Stage 200/ })).toBeEnabled();
    expect(requests.some((request) => request.method !== "GET")).toBe(false);
  });

  it("keeps a cold-start pending-up stage's role outcomes unknown", async () => {
    activeWorkspace = {
      ...workspace,
      checkpoint: {
        accepted_stage: { number: 10, name: "seed" },
        pending_transition: { direction: "up", stage: { number: 200, name: "finish" } },
        workflow_started: true,
      },
      stages: workspace.stages.map((stage) => stage.number === 200
        ? { ...stage, is_accepted_checkpoint: false }
        : stage),
    };
    render(<App />);
    expect((await screen.findAllByText("Pending up")).length).toBeGreaterThanOrEqual(2);
    expect(screen.getByText("Prior execution result is not retained")).toBeInTheDocument();
    expect(screen.getAllByText("Prior outcome unavailable").length).toBeGreaterThanOrEqual(2);
    expect(screen.getAllByText("Unknown")).toHaveLength(3);
    expect(screen.queryByText("Not run")).not.toBeInTheDocument();
  });

  it("uses verify-down retry and reapply labels for an observed pending-down failure", async () => {
    activeWorkspace = {
      ...workspace,
      observation_revision: 4,
      observation: {
        workspace_id: "fixture", operation_id: "old-operation", server_instance_id: "server-one", revision: 4,
        direction: "down", target_stage: 10, state: "stopped", active_role: null, role_results: [],
        omitted_role_results: 0, outputs_evicted: 0,
        confirmed_checkpoint: workspace.checkpoint,
        attempted_checkpoint: null,
        failure: { kind: "process_failed", message: "verify-down exited with status 9", stage: { number: 200, name: "finish" }, role: "verify-down" },
        verification_choices: { retry: { direction: "down", target_stage: 10 }, reverse: { direction: "up", target_stage: 200 } },
      },
    };
    render(<App />);

    expect(await screen.findByRole("button", { name: /Retry verify-down for Stage 200/ })).toBeEnabled();
    expect(screen.getByRole("button", { name: /Reapply Stage 200 upward/ })).toBeEnabled();
    expect(screen.queryByRole("button", { name: /Retry verify-up/ })).not.toBeInTheDocument();
    expect(screen.getByText("Pending · verify-down failed")).toBeInTheDocument();
    expect(screen.getByText("Observed verify-down failed")).toBeInTheDocument();
    expect(screen.queryByText("Prior execution result is not retained")).not.toBeInTheDocument();
  });

  it.each(["verify-up", "verify-down"] as const)("shows retained %s failure evidence for a pending checkpoint", async (role) => {
    const pending = {
      accepted_stage: role === "verify-up" ? { number: 10, name: "seed" } : { number: 200, name: "finish" },
      pending_transition: { direction: role === "verify-up" ? "up" as const : "down" as const, stage: { number: 200, name: "finish" } },
      workflow_started: true,
    };
    const failure: NonNullable<MovementObservation["failure"]> = {
      kind: "process_failed",
      message: `${role} exited with status 9`,
      stage: { number: 200, name: "finish" },
      role,
    };
    activeWorkspace = {
      ...workspace,
      checkpoint: pending,
      selected_stage_number: 200,
      stages: workspace.stages.map((stage) => stage.number === 200
        ? { ...stage, state: "pending", is_accepted_checkpoint: role === "verify-down" }
        : { ...stage, state: role === "verify-up" ? "accepted" : "future", is_accepted_checkpoint: role === "verify-up" }),
      observation_revision: 8,
      observation: {
        workspace_id: "fixture", operation_id: `failed-${role}`, server_instance_id: "server-one", revision: 8,
        direction: role === "verify-up" ? "up" : "down", target_stage: role === "verify-up" ? 200 : 10,
        state: "stopped", active_role: null, role_results: [roleResult(200, role, `output-${role}`, "failed")],
        omitted_role_results: 0, outputs_evicted: 0, confirmed_checkpoint: pending, attempted_checkpoint: null,
        failure,
        verification_choices: {
          retry: { direction: role === "verify-up" ? "up" : "down", target_stage: role === "verify-up" ? 200 : 10 },
          reverse: { direction: role === "verify-up" ? "down" : "up", target_stage: role === "verify-up" ? 10 : 200 },
        },
      },
    };

    render(<App />);

    expect(await screen.findByText(`Pending · ${role} failed`)).toBeInTheDocument();
    expect(screen.getByText(`Observed ${role} failed`)).toBeInTheDocument();
    expect(screen.getAllByText("Process failed · exit 9 · 3 ms").length).toBeGreaterThanOrEqual(2);
    expect(screen.queryByText("Prior execution result is not retained")).not.toBeInTheDocument();
    expect(screen.queryByText("Prior outcome unavailable")).not.toBeInTheDocument();
  });

  it("warns after a failed reverse mutation while keeping existing movement choices available", async () => {
    const pendingUp = {
      accepted_stage: { number: 10, name: "seed" },
      pending_transition: { direction: "up" as const, stage: { number: 200, name: "finish" } },
      workflow_started: true,
    };
    activeWorkspace = {
      ...workspace,
      checkpoint: pendingUp,
      observation_revision: 5,
      observation: {
        workspace_id: "fixture", operation_id: "reverse-failed", server_instance_id: "server-one", revision: 5,
        direction: "down", target_stage: 10, state: "stopped", active_role: null, role_results: [],
        omitted_role_results: 0, outputs_evicted: 0,
        confirmed_checkpoint: pendingUp,
        attempted_checkpoint: null,
        failure: { kind: "process_failed", message: "down exited with status 9", stage: { number: 200, name: "finish" }, role: "down" },
        verification_choices: null,
      },
      stages: workspace.stages.map((stage) => stage.number === 200
        ? { ...stage, state: "pending", is_accepted_checkpoint: false }
        : stage),
    };
    render(<App />);

    expect(await screen.findByRole("alert")).toHaveTextContent("Inspect author-owned effects before continuing");
    expect(screen.getByRole("button", { name: /Continue up toward Stage 200/ })).toBeEnabled();
    expect(screen.getByRole("button", { name: /Continue down toward Stage 10 · seed/ })).toBeEnabled();
    expect(screen.queryByRole("button", { name: /Retry verify-down/ })).not.toBeInTheDocument();
  });

  it("shows one unprepared workspace without hiding the prepared workspace", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByRole("heading", { name: "finish" });
    await user.click(screen.getByRole("button", { name: /unprepared/ }));
    expect((await screen.findAllByText("Workspace unavailable")).length).toBeGreaterThanOrEqual(2);
    expect(screen.getByRole("button", { name: /fixture/ })).toBeInTheDocument();
    expect(screen.getByText(/Prepare storage explicitly/)).toBeInTheDocument();
  });

  it("selects the first usable workspace when startup discovery begins with an unavailable one", async () => {
    const reorderedProject: ProjectView = { ...project, workspaces: [...project.workspaces].reverse() };
    activeProject = reorderedProject;
    render(<App />);
    expect(await screen.findByRole("heading", { name: "finish" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /fixture/ })).toHaveAttribute("aria-current", "page");
  });

  it("refreshes checkpoint details while preserving a manually selected stage", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByRole("heading", { name: "finish" });
    await user.click(screen.getByRole("button", { name: /STAGE 010/ }));
    const source = FakeEventSource.instances.find((item) => item.url.endsWith("/fixture/events"))!;
    source.emit("resync", {
      workspace_id: "fixture", server_instance_id: "server-one", revision: 6, movement_busy: false,
      checkpoint: { accepted_stage: { number: 200, name: "finish" }, pending_transition: null, workflow_started: true },
      observation: null,
    });
    expect(screen.getByRole("heading", { name: "seed" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Refresh workspace status" }));
    await waitFor(() => {
      expect(requests.filter((request) => request.path === "/api/workspaces/fixture")).toHaveLength(2);
    });
    expect(screen.getByRole("heading", { name: "seed" })).toBeInTheDocument();
  });

  it("updates the observing tab's rail from a terminal resync and protects it from a delayed project read", async () => {
    const user = userEvent.setup();
    const baseline = { accepted_stage: null, pending_transition: null, workflow_started: false };
    const staleProject: ProjectView = {
      ...project,
      workspaces: project.workspaces.map((item) => item.id === "fixture"
        ? { ...item, accepted_stage: null, pending_transition: null }
        : item),
    };
    activeProject = staleProject;
    activeWorkspace = {
      ...workspace,
      checkpoint: baseline,
      observation: null,
      selected_stage_number: 10,
      stages: workspace.stages.map((stage) => ({ ...stage, state: "future", is_accepted_checkpoint: false })),
    };
    const delayedProjectRead = deferred<Response>();
    const originalFetch = mockFetch.getMockImplementation() as FetchImplementation;
    let projectReads = 0;
    mockFetch.mockImplementation((input, init) => {
      if (String(input) === "/api/project") {
        projectReads += 1;
        if (projectReads === 2) return delayedProjectRead.promise;
      }
      return originalFetch(input, init);
    });
    render(<App />);
    await screen.findByRole("button", { name: /Advance to Stage 10/ });
    await user.click(screen.getByRole("button", { name: "Refresh workspace status" }));
    await waitFor(() => expect(projectReads).toBe(2));

    const checkpoint = { accepted_stage: { number: 200, name: "finish" }, pending_transition: null, workflow_started: true };
    const completed: MovementObservation = {
      workspace_id: "fixture", operation_id: "other-tab-completed", server_instance_id: "server-one", revision: 20,
      direction: "up", target_stage: 200, state: "complete", active_role: null,
      role_results: [roleResult(200, "up", "other-tab-output")], omitted_role_results: 0, outputs_evicted: 0,
      confirmed_checkpoint: checkpoint, attempted_checkpoint: null, failure: null, verification_choices: null,
    };
    const source = FakeEventSource.instances.find((item) => item.url.endsWith("/fixture/events"))!;
    source.emit("resync", {
      workspace_id: "fixture", server_instance_id: "server-one", revision: 20, movement_busy: false,
      checkpoint, observation: completed,
    });

    expect(await screen.findByRole("button", { name: /Applied · 200/ })).toBeInTheDocument();
    expect(screen.getByText("200 · finish")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Back out Stage 200 to Stage 10/ })).toBeEnabled();
    delayedProjectRead.resolve(jsonResponse(staleProject));
    await waitFor(() => expect(screen.getByRole("button", { name: "Refresh workspace status" })).toBeEnabled());
    expect(screen.getByRole("button", { name: /Applied · 200/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Back out Stage 200 to Stage 10/ })).toBeEnabled();
  });

  it("updates the rail and central controls after reconnecting to a newer checkpoint", async () => {
    const baseline = { accepted_stage: null, pending_transition: null, workflow_started: false };
    activeWorkspace = {
      ...workspace,
      checkpoint: baseline,
      observation: null,
      selected_stage_number: 10,
      stages: workspace.stages.map((stage) => ({ ...stage, state: "future", is_accepted_checkpoint: false })),
    };
    render(<App />);
    await screen.findByRole("button", { name: /Advance to Stage 10/ });
    const checkpoint = { accepted_stage: { number: 10, name: "seed" }, pending_transition: null, workflow_started: true };
    activeWorkspace = {
      ...activeWorkspace,
      checkpoint,
      observation_revision: 9,
      observation: {
        workspace_id: "fixture", operation_id: "second-tab-finished", server_instance_id: "server-one", revision: 9,
        direction: "up", target_stage: 10, state: "complete", active_role: null,
        role_results: [roleResult(10, "up", "second-tab-output")], omitted_role_results: 0, outputs_evicted: 0,
        confirmed_checkpoint: checkpoint, attempted_checkpoint: null, failure: null, verification_choices: null,
      },
      stages: workspace.stages.map((stage) => ({ ...stage, state: stage.number === 10 ? "accepted" : "future", is_accepted_checkpoint: stage.number === 10 })),
    };
    const source = FakeEventSource.instances.find((item) => item.url.endsWith("/fixture/events"))!;
    act(() => source.onerror?.(new Event("error")));

    expect(await screen.findByRole("button", { name: /Applied · 10/ })).toBeInTheDocument();
    expect(screen.getByText("10 · seed")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Back out Stage 10 to baseline/ })).toBeEnabled();
    expect(screen.queryByRole("button", { name: /Advance to Stage 10/ })).not.toBeInTheDocument();
  });

  it("recovers workspace rail and central controls when a completed movement POST response is lost", async () => {
    const user = userEvent.setup();
    const baseline = { accepted_stage: null, pending_transition: null, workflow_started: false };
    activeWorkspace = {
      ...workspace,
      checkpoint: baseline,
      observation: null,
      selected_stage_number: 10,
      stages: workspace.stages.map((stage) => ({ ...stage, state: "future", is_accepted_checkpoint: false })),
    };
    const completedCheckpoint = { accepted_stage: { number: 10, name: "seed" }, pending_transition: null, workflow_started: true };
    const originalFetch = mockFetch.getMockImplementation() as FetchImplementation;
    mockFetch.mockImplementation((input, init) => {
      if (String(input) === "/api/workspaces/fixture/movements" && init?.method === "POST") {
        activeWorkspace = {
          ...activeWorkspace,
          checkpoint: completedCheckpoint,
          observation_revision: 4,
          observation: {
            workspace_id: "fixture", operation_id: "post-response-lost", server_instance_id: "server-one", revision: 4,
            direction: "up", target_stage: 10, state: "complete", active_role: null,
            role_results: [roleResult(10, "up", "post-lost-output")], omitted_role_results: 0, outputs_evicted: 0,
            confirmed_checkpoint: completedCheckpoint, attempted_checkpoint: null, failure: null, verification_choices: null,
          },
          stages: workspace.stages.map((stage) => ({ ...stage, state: stage.number === 10 ? "accepted" : "future", is_accepted_checkpoint: stage.number === 10 })),
        };
        requests.push({ path: String(input), method: "POST" });
        return Promise.reject(new TypeError("Failed to fetch"));
      }
      return originalFetch(input, init);
    });
    render(<App />);
    await user.click(await screen.findByRole("button", { name: /Advance to Stage 10/ }));

    expect(await screen.findByText("Movement completed")).toBeInTheDocument();
    expect(await screen.findByRole("button", { name: /Applied · 10/ })).toBeInTheDocument();
    expect(screen.getByText("10 · seed")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Back out Stage 10 to baseline/ })).toBeEnabled();
    expect(screen.queryByText(/Movement response lost;/)).not.toBeInTheDocument();
  });

  it("clears the fragment token before exchanging it for a same-origin session cookie", async () => {
    const user = userEvent.setup();
    history.replaceState(null, "", "/#session=one-time-secret");
    render(<App />);
    await screen.findByRole("heading", { name: "finish" });
    await waitFor(() => expect(mockFetch).toHaveBeenCalledWith("/api/session", expect.objectContaining({ method: "POST" })));
    expect(location.hash).toBe("");
    const bootstrap = mockFetch.mock.calls.find(([path]) => path === "/api/session");
    expect(bootstrap?.[1]?.headers).toEqual({ Authorization: "Bearer one-time-secret" });
    expect(await screen.findByRole("heading", { name: "Stage inspector" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Collapse workspace rail" }));
    expect(screen.getByRole("button", { name: "Expand workspace rail" })).toBeInTheDocument();
  });

  it("submits the adjacent sparse-stage target regardless of selected inspection and previews raw output inertly", async () => {
    const user = userEvent.setup();
    activeWorkspace = {
      ...workspace,
      checkpoint: { accepted_stage: { number: 10, name: "seed" }, pending_transition: null, workflow_started: true },
      stages: workspace.stages.map((stage) => ({
        ...stage,
        state: stage.number === 10 ? "accepted" : "future",
        is_accepted_checkpoint: stage.number === 10,
      })),
    };
    movementResponse = {
      workspace_id: "fixture", operation_id: "operation-one", server_instance_id: "server-one", revision: 3,
      direction: "up", target_stage: 200, state: "complete", active_role: null,
      role_results: [{ stage: { number: 200, name: "finish" }, role: "up", state: "succeeded", exit_code: 0,
        message: null, elapsed_ms: 4, output_id: "output-one", output_state: "available", stdout_bytes: 10,
        stderr_bytes: 0, stdout_truncated: false, stderr_truncated: false }],
      omitted_role_results: 0, outputs_evicted: 0,
      confirmed_checkpoint: { accepted_stage: { number: 200, name: "finish" }, pending_transition: null, workflow_started: true },
      attempted_checkpoint: null, failure: null, verification_choices: null,
    };
    render(<App />);
    await screen.findByRole("heading", { name: "seed" });
    await user.click(screen.getByRole("button", { name: /STAGE 200/ }));
    const advance = screen.getByRole("button", { name: /Advance to Stage 200/ });
    expect(advance).toBeEnabled();
    await user.click(advance);

    await waitFor(() => expect(requests.some((request) => request.method === "POST")).toBe(true));
    const post = mockFetch.mock.calls.find(([path, init]) => String(path).endsWith("/movements") && init?.method === "POST");
    expect(JSON.parse(String(post?.[1]?.body))).toEqual({ direction: "up", target_stage: 200 });
    expect((await screen.findAllByText("Process succeeded · 4 ms")).length).toBeGreaterThanOrEqual(2);
    await user.click(screen.getByRole("button", { name: "Preview stdout" }));
    const preview = await screen.findByLabelText("stdout text preview");
    expect(preview).toHaveTextContent("�⟦U+0000⟧<script>");
    expect(preview.querySelector("script")).toBeNull();
    expect(screen.getAllByRole("link", { name: "Download raw bytes" })[0]).toHaveAttribute("href", expect.stringContaining("/outputs/output-one/stdout"));
  });

  it("uses Workbench verifier retry choices and keeps a stopped 2xx outcome from appearing successful", async () => {
    const user = userEvent.setup();
    activeWorkspace = {
      ...workspace,
      checkpoint: { accepted_stage: { number: 10, name: "seed" }, pending_transition: { direction: "up", stage: { number: 200, name: "finish" } }, workflow_started: true },
      observation: {
        workspace_id: "fixture", operation_id: "old-operation", server_instance_id: "server-one", revision: 4,
        direction: "up", target_stage: 200, state: "stopped", active_role: null, role_results: [],
        omitted_role_results: 0, outputs_evicted: 0,
        confirmed_checkpoint: { accepted_stage: { number: 10, name: "seed" }, pending_transition: { direction: "up", stage: { number: 200, name: "finish" } }, workflow_started: true },
        attempted_checkpoint: null,
        failure: { kind: "process_failed", message: "verify-up exited with status 9", stage: { number: 200, name: "finish" }, role: "verify-up" },
        verification_choices: { retry: { direction: "up", target_stage: 200 }, reverse: { direction: "down", target_stage: 10 } },
      },
      observation_revision: 4,
      stages: workspace.stages.map((stage) => stage.number === 200 ? { ...stage, state: "pending" } : stage),
    };
    movementResponse = {
      workspace_id: "fixture", operation_id: "new-operation", server_instance_id: "server-one", revision: 7,
      direction: "up", target_stage: 200, state: "stopped", active_role: null, role_results: [],
      omitted_role_results: 0, outputs_evicted: 0,
      confirmed_checkpoint: { accepted_stage: { number: 10, name: "seed" }, pending_transition: { direction: "up", stage: { number: 200, name: "finish" } }, workflow_started: true },
      attempted_checkpoint: { accepted_stage: { number: 200, name: "finish" }, pending_transition: null, workflow_started: true },
      failure: { kind: "checkpoint_save_failed", message: "could not save workbench state", stage: null, role: null },
      verification_choices: null,
    };
    render(<App />);
    expect(await screen.findByRole("button", { name: /Retry verify-up for Stage 200/ })).toBeEnabled();
    await user.click(screen.getByRole("button", { name: /Retry verify-up for Stage 200/ }));
    await waitFor(() => expect(requests.some((request) => request.method === "POST")).toBe(true));
    const post = mockFetch.mock.calls.find(([path, init]) => String(path).endsWith("/movements") && init?.method === "POST");
    expect(JSON.parse(String(post?.[1]?.body))).toEqual({ direction: "up", target_stage: 200 });
    expect(await screen.findByRole("alert")).toHaveTextContent("Inspect author-owned effects before continuing");
    expect(screen.getByText("Movement stopped at the last confirmed checkpoint")).toBeInTheDocument();
    expect(screen.getByText("Checkpoint was not confirmed")).toBeInTheDocument();
    expect(screen.getByRole("note").querySelectorAll("p")[0]).toHaveTextContent("Accepted position: Stage 200 · finish");
    expect(screen.getByRole("note").querySelectorAll("p")[1]).toHaveTextContent("Pending transition: None");
    expect(screen.getByRole("note").querySelector("small")).toHaveTextContent("Workbench last confirmed: Stage 10 · seed. After an ambiguous save failure, actual database contents may be uncertain.");
  });

  it("shows attempted pending publication values separately from the confirmed baseline", async () => {
    const user = userEvent.setup();
    const baseline = { accepted_stage: null, pending_transition: null, workflow_started: false };
    activeWorkspace = {
      ...workspace,
      checkpoint: baseline,
      observation: null,
      selected_stage_number: 10,
      stages: workspace.stages.map((stage) => ({ ...stage, state: "future", is_accepted_checkpoint: false })),
    };
    movementResponse = {
      workspace_id: "fixture", operation_id: "pending-save-failure", server_instance_id: "server-one", revision: 5,
      direction: "up", target_stage: 10, state: "stopped", active_role: null,
      role_results: [roleResult(10, "up", "pending-output")], omitted_role_results: 0, outputs_evicted: 0,
      confirmed_checkpoint: baseline,
      attempted_checkpoint: {
        accepted_stage: null,
        pending_transition: { direction: "up", stage: { number: 10, name: "seed" } },
        workflow_started: true,
      },
      failure: { kind: "checkpoint_save_failed", message: "pending checkpoint write failed", stage: null, role: null },
      verification_choices: null,
    };
    render(<App />);
    await user.click(await screen.findByRole("button", { name: /Advance to Stage 10/ }));
    expect((await screen.findByRole("note")).querySelectorAll("p")[0]).toHaveTextContent("Accepted position: Baseline · no accepted stages");
    expect(screen.getByRole("note").querySelectorAll("p")[1]).toHaveTextContent("Pending transition: up · Stage 10 · seed");
    expect(screen.getByText("Workbench last confirmed: Baseline · no accepted stages. After an ambiguous save failure, actual database contents may be uncertain.")).toBeInTheDocument();
  });

  it.each(["complete", "stopped"] as const)("keeps delayed %s movement effects scoped while the user switches workspaces and stages", async (state) => {
    const user = userEvent.setup();
    const delayedResponse = deferred<Response>();
    const originalFetch = mockFetch.getMockImplementation() as FetchImplementation;
    mockFetch.mockImplementation((input, init) => {
      if (String(input) === "/api/workspaces/fixture/movements" && init?.method === "POST") {
        requests.push({ path: String(input), method: "POST" });
        return delayedResponse.promise;
      }
      return originalFetch(input, init);
    });
    const accepted = { accepted_stage: { number: 10, name: "seed" }, pending_transition: null, workflow_started: true };
    activeWorkspace = {
      ...workspace,
      checkpoint: accepted,
      observation: null,
      selected_stage_number: 10,
      stages: workspace.stages.map((stage) => ({ ...stage, state: stage.number === 10 ? "accepted" : "future", is_accepted_checkpoint: stage.number === 10 })),
    };
    extraWorkspace = {
      ...activeWorkspace,
      workspace: { id: "sibling", name: "sibling" },
      selected_stage_number: 10,
    };
    activeProject = {
      ...project,
      workspaces: [...project.workspaces, {
        id: "sibling", name: "sibling", available: true, issue: null, stage_count: 2,
        accepted_stage: accepted.accepted_stage, pending_transition: null,
      }],
    };
    render(<App />);
    await user.click(await screen.findByRole("button", { name: /Advance to Stage 200/ }));
    await waitFor(() => expect(requests.some((request) => request.path === "/api/workspaces/fixture/movements" && request.method === "POST")).toBe(true));
    await user.click(screen.getByRole("button", { name: /sibling/ }));
    expect(await screen.findByRole("heading", { name: "seed" })).toBeInTheDocument();

    const checkpoint = state === "complete"
      ? { accepted_stage: { number: 200, name: "finish" }, pending_transition: null, workflow_started: true }
      : accepted;
    const observation: MovementObservation = {
      workspace_id: "fixture", operation_id: `delayed-${state}`, server_instance_id: "server-one", revision: 21,
      direction: "up", target_stage: 200, state, active_role: null,
      role_results: [roleResult(200, "up", `delayed-${state}-output`, state === "complete" ? "succeeded" : "failed")],
      omitted_role_results: 0, outputs_evicted: 0, confirmed_checkpoint: checkpoint, attempted_checkpoint: null,
      failure: state === "stopped" ? { kind: "process_failed", message: "A only failure", stage: { number: 200, name: "finish" }, role: "up" } : null,
      verification_choices: null,
    };
    await act(async () => {
      delayedResponse.resolve(jsonResponse({ observation }));
      await new Promise((resolve) => setTimeout(resolve, 0));
    });
    expect(screen.getByRole("heading", { name: "seed" })).toBeInTheDocument();
    expect(screen.queryByText("A only failure")).not.toBeInTheDocument();
    expect(screen.queryByText("Movement completed")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /sibling/ })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("button", { name: /Advance to Stage 200/ })).toBeEnabled();
  });

  it("keeps an early terminal SSE snapshot ahead of a delayed stale initial GET", async () => {
    const delayedGet = deferred<Response>();
    const originalFetch = mockFetch.getMockImplementation() as FetchImplementation;
    mockFetch.mockImplementation(async (input, init) => String(input) === "/api/workspaces/fixture"
      ? delayedGet.promise
      : originalFetch(input, init));
    const staleView: WorkspaceView = {
      ...workspace,
      observation_revision: 0,
      observation: null,
      checkpoint: { accepted_stage: null, pending_transition: null, workflow_started: false },
      selected_stage_number: 10,
      stages: workspace.stages.map((stage) => ({ ...stage, state: "future", is_accepted_checkpoint: false })),
    };
    render(<App />);
    await waitFor(() => expect(FakeEventSource.instances.some((item) => item.url.endsWith("/fixture/events"))).toBe(true));
    const source = FakeEventSource.instances.find((item) => item.url.endsWith("/fixture/events"))!;
    const checkpoint = { accepted_stage: { number: 200, name: "finish" }, pending_transition: null, workflow_started: true };
    const finished: MovementObservation = {
      workspace_id: "fixture", operation_id: "finished-before-get", server_instance_id: "server-one", revision: 11,
      direction: "up", target_stage: 200, state: "complete", active_role: null, role_results: [],
      omitted_role_results: 0, outputs_evicted: 0, confirmed_checkpoint: checkpoint, attempted_checkpoint: null,
      failure: null, verification_choices: null,
    };
    source.emit("resync", {
      workspace_id: "fixture", server_instance_id: "server-one", revision: 11, movement_busy: false,
      checkpoint, observation: finished,
    });
    delayedGet.resolve(jsonResponse(staleView));

    expect(await screen.findByText("Movement completed")).toBeInTheDocument();
    expect(screen.getByText("200 · finish")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Back out Stage 200 to Stage 10/ })).toBeEnabled();
    expect(await screen.findByRole("heading", { name: "finish" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Advance to Stage 10/ })).not.toBeInTheDocument();
  });

  it("reconciles missed completion from an SSE resync snapshot", async () => {
    render(<App />);
    await screen.findByRole("heading", { name: "finish" });
    const source = FakeEventSource.instances.find((item) => item.url.endsWith("/fixture/events"))!;
    const checkpoint = { accepted_stage: { number: 200, name: "finish" }, pending_transition: null, workflow_started: true };
    const finished: MovementObservation = {
      workspace_id: "fixture", operation_id: "missed-completion", server_instance_id: "server-one", revision: 12,
      direction: "up", target_stage: 200, state: "complete", active_role: null, role_results: [],
      omitted_role_results: 0, outputs_evicted: 0, confirmed_checkpoint: checkpoint, attempted_checkpoint: null,
      failure: null, verification_choices: null,
    };
    source.emit("resync", {
      workspace_id: "fixture", server_instance_id: "server-one", revision: 12, movement_busy: false,
      checkpoint, observation: finished,
    });
    expect(await screen.findByText("Movement completed")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Back out Stage 200 to Stage 10/ })).toBeEnabled();
    expect(screen.queryByRole("button", { name: /Continue pending down/ })).not.toBeInTheDocument();
  });

  it("rejects a delayed GET older than a newer SSE invalidation revision", async () => {
    const delayedRead = deferred<Response>();
    const originalFetch = mockFetch.getMockImplementation() as FetchImplementation;
    const checkpoint = { accepted_stage: { number: 200, name: "finish" }, pending_transition: null, workflow_started: true };
    const finished: MovementObservation = {
      workspace_id: "fixture", operation_id: "event-newer-than-get", server_instance_id: "server-one", revision: 12,
      direction: "up", target_stage: 200, state: "complete", active_role: null, role_results: [],
      omitted_role_results: 0, outputs_evicted: 0, confirmed_checkpoint: checkpoint, attempted_checkpoint: null,
      failure: null, verification_choices: null,
    };
    const latestView: WorkspaceView = {
      ...workspace,
      observation_revision: 12,
      observation: finished,
      checkpoint,
      stages: workspace.stages.map((stage) => ({ ...stage, state: "accepted", is_accepted_checkpoint: stage.number === 200 })),
    };
    let workspaceReads = 0;
    mockFetch.mockImplementation((input, init) => {
      if (String(input) === "/api/workspaces/fixture" && init?.method !== "POST") {
        workspaceReads += 1;
        if (workspaceReads === 2) {
          requests.push({ path: String(input), method: "GET" });
          return delayedRead.promise;
        }
        if (workspaceReads > 2) return Promise.resolve(jsonResponse(latestView));
      }
      return originalFetch(input, init);
    });
    render(<App />);
    await screen.findByRole("heading", { name: "finish" });
    const source = FakeEventSource.instances.find((item) => item.url.endsWith("/fixture/events"))!;
    source.emit("movement.finished", {
      workspace_id: "fixture", server_instance_id: "server-one", revision: 12,
      operation_id: "event-newer-than-get", kind: "movement.finished",
    });
    await waitFor(() => expect(workspaceReads).toBe(2));
    delayedRead.resolve(jsonResponse({ ...workspace, observation_revision: 0, observation: null }));

    expect(await screen.findByText("Movement completed")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Back out Stage 200 to Stage 10/ })).toBeEnabled();
    expect(workspaceReads).toBeGreaterThanOrEqual(3);
  });

  it("scopes output preview state to stage and discards a late fetch after inspection changes", async () => {
    const user = userEvent.setup();
    const delayedOldOutput = deferred<Response>();
    activeWorkspace = {
      ...workspace,
      observation_revision: 14,
      observation: {
        workspace_id: "fixture", operation_id: "two-stage-output", server_instance_id: "server-one", revision: 14,
        direction: "up", target_stage: 200, state: "complete", active_role: null,
        role_results: [roleResult(10, "up", "output-10"), roleResult(200, "up", "output-200")],
        omitted_role_results: 0, outputs_evicted: 0, confirmed_checkpoint: workspace.checkpoint,
        attempted_checkpoint: null, failure: null, verification_choices: null,
      },
    };
    outputResponse = (path) => path.includes("output-10")
      ? delayedOldOutput.promise
      : Promise.resolve(new Response("stage 200 bytes", { status: 200 }));
    render(<App />);
    const userStage10 = await screen.findByRole("button", { name: /STAGE 010/ });
    await user.click(userStage10);
    await user.click(await screen.findByRole("button", { name: "Preview stdout" }));
    await waitFor(() => expect(requests.some((request) => request.path.includes("output-10") && request.path.endsWith("/stdout"))).toBe(true));
    await user.click(screen.getByRole("button", { name: /STAGE 200/ }));
    expect(await screen.findByRole("heading", { name: "finish" })).toBeInTheDocument();
    delayedOldOutput.resolve(new Response("stale Stage 10 bytes", { status: 200 }));
    expect(await screen.findByRole("button", { name: "Preview stdout" })).toBeInTheDocument();
    expect(screen.queryByText("stale Stage 10 bytes")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Preview stdout" }));
    expect(await screen.findByLabelText("stdout text preview")).toHaveTextContent("stage 200 bytes");
  });

  it("does not reuse a preview when switching workspaces with matching output identities", async () => {
    const user = userEvent.setup();
    const observed = {
      workspace_id: "fixture", operation_id: "same-operation", server_instance_id: "server-one", revision: 15,
      direction: "up" as const, target_stage: 200, state: "complete" as const, active_role: null,
      role_results: [roleResult(200, "up", "same-output")], omitted_role_results: 0, outputs_evicted: 0,
      confirmed_checkpoint: workspace.checkpoint, attempted_checkpoint: null, failure: null, verification_choices: null,
    };
    activeWorkspace = { ...workspace, observation_revision: 15, observation: observed };
    extraWorkspace = {
      ...workspace,
      workspace: { id: "sibling", name: "sibling" },
      observation_revision: 15,
      observation: { ...observed, workspace_id: "sibling" },
    };
    activeProject = {
      ...project,
      workspaces: [...project.workspaces, {
        id: "sibling", name: "sibling", available: true, issue: null, stage_count: 2,
        accepted_stage: { number: 200, name: "finish" }, pending_transition: null,
      }],
    };
    outputResponse = (path) => Promise.resolve(new Response(path.includes("/sibling/") ? "sibling bytes" : "fixture bytes", { status: 200 }));
    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Preview stdout" }));
    expect(await screen.findByLabelText("stdout text preview")).toHaveTextContent("fixture bytes");
    await user.click(screen.getByRole("button", { name: /sibling/ }));
    await waitFor(() => expect(screen.getByRole("button", { name: /sibling/ })).toHaveAttribute("aria-current", "page"));
    expect(await screen.findByRole("heading", { name: "finish" })).toBeInTheDocument();
    expect(screen.queryByText("fixture bytes")).not.toBeInTheDocument();
    await user.click(await screen.findByRole("button", { name: "Preview stdout" }));
    expect(await screen.findByLabelText("stdout text preview")).toHaveTextContent("sibling bytes");
  });

  it("resets captured output previews after a successive movement of the same role", async () => {
    const user = userEvent.setup();
    const pending = { accepted_stage: { number: 10, name: "seed" }, pending_transition: { direction: "up" as const, stage: { number: 200, name: "finish" } }, workflow_started: true };
    activeWorkspace = {
      ...workspace,
      checkpoint: pending,
      observation_revision: 16,
      stages: workspace.stages.map((stage) => stage.number === 200 ? { ...stage, state: "pending" } : stage),
      observation: {
        workspace_id: "fixture", operation_id: "first-verify", server_instance_id: "server-one", revision: 16,
        direction: "up", target_stage: 200, state: "stopped", active_role: null,
        role_results: [{ ...roleResult(200, "verify-up", "old-verify-output", "failed"), message: "failed" }],
        omitted_role_results: 0, outputs_evicted: 0, confirmed_checkpoint: pending, attempted_checkpoint: null,
        failure: { kind: "process_failed", message: "verification failed", stage: { number: 200, name: "finish" }, role: "verify-up" },
        verification_choices: { retry: { direction: "up", target_stage: 200 }, reverse: { direction: "down", target_stage: 10 } },
      },
    };
    movementResponse = {
      workspace_id: "fixture", operation_id: "second-verify", server_instance_id: "server-one", revision: 20,
      direction: "up", target_stage: 200, state: "complete", active_role: null,
      role_results: [roleResult(200, "verify-up", "new-verify-output")], omitted_role_results: 0, outputs_evicted: 0,
      confirmed_checkpoint: { accepted_stage: { number: 200, name: "finish" }, pending_transition: null, workflow_started: true },
      attempted_checkpoint: null, failure: null, verification_choices: null,
    };
    outputResponse = (path) => Promise.resolve(new Response(path.includes("old-verify-output") ? "first attempt bytes" : "second attempt bytes", { status: 200 }));
    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Preview stdout" }));
    expect(await screen.findByLabelText("stdout text preview")).toHaveTextContent("first attempt bytes");
    await user.click(screen.getByRole("button", { name: /Retry verify-up for Stage 200/ }));
    await waitFor(() => expect(screen.queryByText("first attempt bytes")).not.toBeInTheDocument());
    await user.click(await screen.findByRole("button", { name: "Preview stdout" }));
    expect(await screen.findByLabelText("stdout text preview")).toHaveTextContent("second attempt bytes");
  });

  it("reconciles SSE snapshots by server incarnation and revision", async () => {
    render(<App />);
    await screen.findByRole("heading", { name: "finish" });
    const source = FakeEventSource.instances.find((item) => item.url.endsWith("/fixture/events"));
    expect(source).toBeDefined();
    const running: MovementObservation = {
      workspace_id: "fixture", operation_id: "operation-live", server_instance_id: "server-one", revision: 9,
      direction: "up", target_stage: 200, state: "running",
      active_role: { stage: { number: 200, name: "finish" }, role: "verify-up" }, role_results: [],
      omitted_role_results: 0, outputs_evicted: 0, confirmed_checkpoint: null, attempted_checkpoint: null,
      failure: null, verification_choices: null,
    };
    source?.emit("snapshot", {
      workspace_id: "fixture", server_instance_id: "server-one", revision: 9, movement_busy: true,
      checkpoint: workspace.checkpoint, observation: running,
    });
    expect(await screen.findByText("Movement is in progress")).toBeInTheDocument();
    source?.emit("snapshot", {
      workspace_id: "fixture", server_instance_id: "server-one", revision: 8, movement_busy: false,
      checkpoint: workspace.checkpoint, observation: null,
    });
    expect(screen.getByText("Movement is in progress")).toBeInTheDocument();
    source?.emit("snapshot", {
      workspace_id: "fixture", server_instance_id: "another-server", revision: 99, movement_busy: false,
      checkpoint: workspace.checkpoint, observation: null,
    });
    expect(screen.getByText("Movement is in progress")).toBeInTheDocument();
  });
});
