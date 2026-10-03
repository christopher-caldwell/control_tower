import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./workbench";
import type { DefinitionView, MovementObservation, ProjectView, RuntimeSnapshot, WorkspaceView } from "./types";

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
let movementResponse: MovementObservation | null;

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

beforeEach(() => {
  requests = [];
  activeProject = project;
  activeWorkspace = workspace;
  movementResponse = null;
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
    expect(screen.getByText("Pending · outcome unknown")).toBeInTheDocument();
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
    await user.click(screen.getByRole("button", { name: "Refresh workspace status" }));
    await waitFor(() => {
      expect(requests.filter((request) => request.path === "/api/workspaces/fixture")).toHaveLength(2);
    });
    expect(screen.getByRole("heading", { name: "seed" })).toBeInTheDocument();
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
    await screen.findByRole("heading", { name: "finish" });
    await user.click(screen.getByRole("button", { name: /STAGE 010/ }));
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
      workspace_id: "fixture", server_instance_id: "server-one", revision: 9, movement_busy: true, observation: running,
    });
    expect(await screen.findByText("Movement is in progress")).toBeInTheDocument();
    source?.emit("snapshot", {
      workspace_id: "fixture", server_instance_id: "server-one", revision: 8, movement_busy: false, observation: null,
    });
    expect(screen.getByText("Movement is in progress")).toBeInTheDocument();
    source?.emit("snapshot", {
      workspace_id: "fixture", server_instance_id: "another-server", revision: 99, movement_busy: false, observation: null,
    });
    expect(screen.getByText("Movement is in progress")).toBeInTheDocument();
  });
});
