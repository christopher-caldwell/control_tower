import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./workbench";
import type { DefinitionView, ProjectView, WorkspaceView } from "./types";

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

function jsonResponse(value: unknown) {
  return new Response(JSON.stringify(value), { status: 200, headers: { "Content-Type": "application/json" } });
}

beforeEach(() => {
  requests = [];
  activeProject = project;
  activeWorkspace = workspace;
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
    return new Response("not found", { status: 404 });
  });
  vi.stubGlobal("fetch", mockFetch);
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("read-only desktop workbench", () => {
  it("keeps pending state distinct from the accepted checkpoint and renders definitions inertly", async () => {
    render(<App />);
    expect(await screen.findByRole("heading", { name: "finish" })).toBeInTheDocument();
    expect(screen.getAllByText("Pending down").length).toBeGreaterThanOrEqual(2);
    expect(screen.getByText("This is also the last confirmed checkpoint. The pending transition has not been accepted.")).toBeInTheDocument();
    expect(screen.getByText("Pending · outcome unknown")).toBeInTheDocument();
    expect(screen.getAllByText("Unknown").length).toBeGreaterThanOrEqual(3);
    expect(screen.getAllByText("No retained result").length).toBeGreaterThanOrEqual(3);
    expect(screen.queryByText("Not run")).not.toBeInTheDocument();

    const source = await screen.findByText(/<script>alert\(1\)<\/script>/);
    expect(source.tagName.toLowerCase()).toBe("pre");
    expect(document.querySelector("pre script")).toBeNull();
    expect(screen.getByRole("button", { name: /Advance to next stage/ })).toBeDisabled();
    expect(requests.every((request) => request.method === "GET")).toBe(true);
  });

  it("allows read-only stage inspection and manual rail collapse without retargeting actions", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByRole("heading", { name: "finish" });

    await user.click(screen.getByRole("button", { name: "Collapse stage inspector" }));
    expect(screen.getByRole("button", { name: "Expand stage inspector" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /STAGE 010/ }));
    expect(screen.getByRole("button", { name: "Expand stage inspector" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Advance to next stage/ })).toBeDisabled();

    await user.click(screen.getByRole("button", { name: "Expand stage inspector" }));
    expect(await screen.findByText("Accepted earlier · execution evidence not retained")).toBeInTheDocument();
    expect(screen.getAllByText("No retained result")).toHaveLength(2);
    expect(screen.getAllByText("Unknown")).toHaveLength(2);
    expect(screen.getByRole("button", { name: /Advance to next stage/ })).toBeDisabled();
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
    expect(screen.getAllByText("No retained result")).toHaveLength(3);
    expect(screen.getAllByText("Unknown")).toHaveLength(3);
    expect(screen.queryByText("Not run")).not.toBeInTheDocument();
  });

  it("shows one unprepared workspace without hiding the prepared workspace", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByRole("heading", { name: "finish" });
    await user.click(screen.getByRole("button", { name: /unprepared/ }));
    expect(await screen.findByText("Workspace unavailable")).toBeInTheDocument();
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
});
