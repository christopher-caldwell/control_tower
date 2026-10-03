import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "./workbench";
import type { MovementObservation, ProjectView, RoleObservation, WorkspaceView } from "./types";

const stage = { number: 10, name: "seed" };

class ControlledEventSource extends EventTarget {
  static instances: ControlledEventSource[] = [];

  constructor(readonly url: string, _init?: EventSourceInit) {
    super();
    ControlledEventSource.instances.push(this);
  }

  close() {}

  snapshot(value: WorkspaceView) {
    this.dispatchEvent(new MessageEvent("snapshot", { data: JSON.stringify(value) }));
  }
}

type OutputReader = (url: string, signal?: AbortSignal) => Promise<Response>;

function result(overrides: Partial<RoleObservation> = {}): RoleObservation {
  return {
    stage,
    role: "up",
    state: "succeeded",
    exit_code: 0,
    message: null,
    elapsed_ms: 5,
    output_available: true,
    stdout_bytes: 10,
    stderr_bytes: 8,
    ...overrides,
  };
}

function observation(operationId: string, roleResults: RoleObservation[] = [result()]): MovementObservation {
  return {
    operation_id: operationId,
    direction: "up",
    target_stage: 10,
    state: "complete",
    active_role: null,
    role_results: roleResults,
    confirmed_checkpoint: null,
    attempted_checkpoint: null,
    failure: null,
    verification_choices: null,
  };
}

function workspace(id: string, operationId = "op-A", roleResults = [result()]): WorkspaceView {
  return {
    project_name: "preview-fixture",
    workspace: { id, name: id },
    current_status: "available",
    status_issue: null,
    checkpoint: {
      accepted_stage: stage,
      pending_transition: null,
      workflow_started: true,
      state: { completed_stage_count: 1, uuid: "fixture-run", pending: null },
    },
    movement_choices: [],
    selected_stage_number: 10,
    stages: [{ number: 10, name: "seed", state: "accepted", is_accepted_checkpoint: true, definitions: [{ role: "up", path: "stages/010-seed/up" }] }],
    movement_busy: false,
    observation: observation(operationId, roleResults),
  };
}

function projectFor(ids: string[]): ProjectView {
  return {
    name: "preview-fixture",
    workspace_root: "workspaces/",
    workspaces: ids.map((id) => ({
      id,
      name: id,
      available: true,
      issue: null,
      stage_count: 1,
      accepted_stage: stage,
      pending_transition: null,
    })),
  };
}

function installWorkbench(snapshots: Record<string, WorkspaceView>, readOutput: OutputReader) {
  ControlledEventSource.instances = [];
  const outputFetch = vi.fn((url: string, signal?: AbortSignal) => readOutput(url, signal));
  const fetchMock = vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input);
    if (url === "/api/project") return Promise.resolve(Response.json(projectFor(Object.keys(snapshots))));
    if (url.includes("/outputs/")) return outputFetch(url, init?.signal ?? undefined);
    if (/\/stages\/\d+$/.test(url)) return Promise.resolve(Response.json({ stage, definitions: [] }));
    return Promise.resolve(Response.json({}, { status: 404 }));
  });
  vi.stubGlobal("fetch", fetchMock);
  vi.stubGlobal("EventSource", ControlledEventSource);
  render(<App />);
  return { fetchMock, outputFetch };
}

async function emitSnapshot(id: string, snapshot: WorkspaceView) {
  await waitFor(() => expect(ControlledEventSource.instances.some((source) => source.url.includes("/api/workspaces/" + id + "/events"))).toBe(true));
  const source = ControlledEventSource.instances.find((item) => item.url.includes("/api/workspaces/" + id + "/events"));
  if (!source) throw new Error("No event source for " + id);
  act(() => source.snapshot(snapshot));
  await screen.findByRole("heading", { name: "Stage inspector" });
}

function outputLink(stream: "stdout" | "stderr" = "stdout") {
  return screen.getAllByRole("link", { name: "Download raw bytes" }).find((link) => link.getAttribute("href")?.endsWith("/" + stream));
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  ControlledEventSource.instances = [];
});

describe("rendered captured-output identity", () => {
  it("replaces completed-operation output, preserves same-identity previews, and keeps streams separate", async () => {
    const user = userEvent.setup();
    const snapshotA = workspace("workspace-a", "op-A");
    const { outputFetch } = installWorkbench({ "workspace-a": snapshotA }, async (url) => new Response(url.includes("/stderr") ? "A stderr" : url.includes("op-A") ? "A stdout" : "B stdout"));
    await emitSnapshot("workspace-a", snapshotA);

    await user.click(screen.getByRole("button", { name: "Preview stdout" }));
    expect(await screen.findByLabelText("stdout text preview")).toHaveTextContent("A stdout");
    expect(outputLink()?.getAttribute("href")).toBe("/api/workspaces/workspace-a/outputs/op-A/0/stdout");

    const sameIdentity = workspace("workspace-a", "op-A");
    sameIdentity.observation!.role_results[0]!.stdout_bytes = 99;
    await emitSnapshot("workspace-a", sameIdentity);
    expect(screen.getByLabelText("stdout text preview")).toHaveTextContent("A stdout");

    const snapshotB = workspace("workspace-a", "op-B");
    await emitSnapshot("workspace-a", snapshotB);
    expect(screen.queryByText("A stdout")).not.toBeInTheDocument();
    expect(outputLink()?.getAttribute("href")).toBe("/api/workspaces/workspace-a/outputs/op-B/0/stdout");
    expect(screen.getByRole("button", { name: "Preview stdout" })).toBeEnabled();
    expect(screen.queryByLabelText("stdout text preview")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Preview stdout" }));
    expect(await screen.findByLabelText("stdout text preview")).toHaveTextContent("B stdout");
    await user.click(screen.getByRole("button", { name: "Preview stderr" }));
    expect(await screen.findByLabelText("stderr text preview")).toHaveTextContent("A stderr");
    expect(screen.getByLabelText("stdout text preview")).toHaveTextContent("B stdout");
    expect(outputFetch.mock.calls.map(([url]) => url)).toEqual([
      "/api/workspaces/workspace-a/outputs/op-A/0/stdout",
      "/api/workspaces/workspace-a/outputs/op-B/0/stdout",
      "/api/workspaces/workspace-a/outputs/op-B/0/stderr",
    ]);
  });

  it("clears an output-read error when its identity is replaced", async () => {
    const user = userEvent.setup();
    const snapshotA = workspace("workspace-a", "op-A");
    installWorkbench({ "workspace-a": snapshotA }, async (url) => url.includes("op-A")
      ? new Response(null, { status: 503 })
      : new Response("B stdout"));
    await emitSnapshot("workspace-a", snapshotA);
    await user.click(screen.getByRole("button", { name: "Preview stdout" }));
    expect(await screen.findByText("Output read failed (503).")).toBeInTheDocument();

    await emitSnapshot("workspace-a", workspace("workspace-a", "op-B"));
    expect(screen.queryByText("Output read failed (503).")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Preview stdout" })).toBeEnabled();
  });

  it("renders preview content as inert text and keeps the raw download target", async () => {
    const user = userEvent.setup();
    const snapshot = workspace("workspace-a", "op-A");
    installWorkbench({ "workspace-a": snapshot }, async () => new Response("<script>window.fixture=true</script>\u001b[31mraw"));
    await emitSnapshot("workspace-a", snapshot);

    await user.click(screen.getByRole("button", { name: "Preview stdout" }));
    const preview = await screen.findByLabelText("stdout text preview");
    expect(preview).toHaveTextContent("<script>window.fixture=true</script>␛[31mraw");
    expect(preview.querySelector("script")).toBeNull();
    expect(outputLink()?.getAttribute("href")).toBe("/api/workspaces/workspace-a/outputs/op-A/0/stdout");
    expect(outputLink()).toHaveAttribute("download", "up-stdout.bin");
  });

  it("resets when the workspace identity changes with the same operation and result index", async () => {
    const user = userEvent.setup();
    const snapshotA = workspace("workspace-a", "same-operation");
    const snapshotB = workspace("workspace-b", "same-operation");
    installWorkbench({ "workspace-a": snapshotA, "workspace-b": snapshotB }, async (url) => new Response(url.includes("workspace-a") ? "workspace A bytes" : "workspace B bytes"));
    await emitSnapshot("workspace-a", snapshotA);
    await user.click(screen.getByRole("button", { name: "Preview stdout" }));
    expect(await screen.findByLabelText("stdout text preview")).toHaveTextContent("workspace A bytes");

    await user.click(screen.getByRole("button", { name: /workspace-b/ }));
    await emitSnapshot("workspace-b", snapshotB);
    expect(screen.queryByText("workspace A bytes")).not.toBeInTheDocument();
    expect(outputLink()?.getAttribute("href")).toBe("/api/workspaces/workspace-b/outputs/same-operation/0/stdout");
    expect(screen.getByRole("button", { name: "Preview stdout" })).toBeEnabled();
  });

  it("resets when the role-result index changes", async () => {
    const user = userEvent.setup();
    const snapshotA = workspace("workspace-a", "same-operation");
    installWorkbench({ "workspace-a": snapshotA }, async (url) => new Response(url.endsWith("/1/stdout") ? "index one" : "index zero"));
    await emitSnapshot("workspace-a", snapshotA);
    await user.click(screen.getByRole("button", { name: "Preview stdout" }));
    expect(await screen.findByLabelText("stdout text preview")).toHaveTextContent("index zero");

    const precedingResult = result({ stage: { number: 5, name: "prior" }, role: "down" });
    const shifted = workspace("workspace-a", "same-operation", [precedingResult, result()]);
    await emitSnapshot("workspace-a", shifted);
    expect(screen.queryByText("index zero")).not.toBeInTheDocument();
    expect(outputLink()?.getAttribute("href")).toBe("/api/workspaces/workspace-a/outputs/same-operation/1/stdout");
    expect(screen.getByRole("button", { name: "Preview stdout" })).toBeEnabled();
  });

  it("uses the same identity reset in unavailable-status diagnostics", async () => {
    const user = userEvent.setup();
    const snapshotA = workspace("workspace-a", "op-A");
    snapshotA.current_status = "unavailable";
    snapshotA.status_issue = "checkpoint read failed";
    snapshotA.checkpoint = null;
    snapshotA.stages = [];
    snapshotA.movement_choices = [];
    installWorkbench({ "workspace-a": snapshotA }, async (url) => new Response(url.includes("op-A") ? "diagnostic A" : "diagnostic B"));
    await emitSnapshot("workspace-a", snapshotA);
    await user.click(screen.getByRole("button", { name: "Preview stdout" }));
    expect(await screen.findByLabelText("stdout text preview")).toHaveTextContent("diagnostic A");

    const snapshotB = workspace("workspace-a", "op-B");
    snapshotB.current_status = "unavailable";
    snapshotB.status_issue = "checkpoint read failed";
    snapshotB.checkpoint = null;
    snapshotB.stages = [];
    snapshotB.movement_choices = [];
    await emitSnapshot("workspace-a", snapshotB);
    expect(screen.queryByText("diagnostic A")).not.toBeInTheDocument();
    expect(outputLink()?.getAttribute("href")).toBe("/api/workspaces/workspace-a/outputs/op-B/0/stdout");
    expect(screen.getByRole("button", { name: "Preview stdout" })).toBeEnabled();
  });

  it.each(["late success", "late error"] as const)("ignores an aborted A read that settles as a %s after B starts reading", async (settlement) => {
    const user = userEvent.setup();
    const pendingA = deferred<Response>();
    const pendingB = deferred<Response>();
    const snapshotA = workspace("workspace-a", "op-A");
    const { outputFetch } = installWorkbench({ "workspace-a": snapshotA }, (url) => url.includes("op-A") ? pendingA.promise : pendingB.promise);
    await emitSnapshot("workspace-a", snapshotA);
    await user.click(screen.getByRole("button", { name: "Preview stdout" }));
    await waitFor(() => expect(outputFetch).toHaveBeenCalledTimes(1));
    const oldSignal = outputFetch.mock.calls[0]?.[1];
    expect(screen.getByRole("button", { name: "Reading…" })).toBeDisabled();

    const snapshotB = workspace("workspace-a", "op-B");
    await emitSnapshot("workspace-a", snapshotB);
    expect(oldSignal?.aborted).toBe(true);
    expect(screen.getByRole("button", { name: "Preview stdout" })).toBeEnabled();
    await user.click(screen.getByRole("button", { name: "Preview stdout" }));
    await waitFor(() => expect(outputFetch).toHaveBeenCalledTimes(2));
    expect(screen.getByRole("button", { name: "Reading…" })).toBeDisabled();

    await act(async () => {
      if (settlement === "late success") pendingA.resolve(new Response("late A bytes"));
      else pendingA.reject(new Error("late A failure"));
      await new Promise((resolve) => setTimeout(resolve, 0));
    });
    expect(screen.getByRole("button", { name: "Reading…" })).toBeDisabled();
    expect(screen.queryByText("late A bytes")).not.toBeInTheDocument();
    expect(screen.queryByText("late A failure")).not.toBeInTheDocument();
    expect(screen.queryByText(/Output read failed/)).not.toBeInTheDocument();

    await act(async () => {
      pendingB.resolve(new Response("B bytes"));
      await pendingB.promise;
    });
    expect(await screen.findByLabelText("stdout text preview")).toHaveTextContent("B bytes");
    expect(screen.getByRole("button", { name: "Reload stdout" })).toBeEnabled();
  });
});

describe("rendered output availability wording", () => {
  it("distinguishes pending output from an actual launch failure", async () => {
    const pending = result({ state: "in_progress", exit_code: null, message: null, output_available: false, stdout_bytes: 0, stderr_bytes: 0 });
    const snapshot = workspace("workspace-a", "op-A", [pending]);
    installWorkbench({ "workspace-a": snapshot }, async () => new Response(null, { status: 404 }));
    await emitSnapshot("workspace-a", snapshot);
    expect(screen.getByText("Captured output will be available when the role returns.")).toBeInTheDocument();
    expect(screen.queryByText(/could not be started/)).not.toBeInTheDocument();
    expect(screen.queryByText(/launched successfully/)).not.toBeInTheDocument();

    const launchFailed = result({ state: "launch_failed", exit_code: null, message: "permission denied by fixture", output_available: false, stdout_bytes: 0, stderr_bytes: 0 });
    await emitSnapshot("workspace-a", workspace("workspace-a", "op-B", [launchFailed]));
    const capturedFailure = screen.getByText("Output unavailable because the executable could not be started.").closest(".captured-role");
    expect(capturedFailure).toHaveTextContent("permission denied by fixture");
    expect(screen.queryByText("Captured output will be available when the role returns.")).not.toBeInTheDocument();
  });
});

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}
