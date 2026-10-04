import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "./workbench";
import type { MovementObservation, WorkspaceView, RoleObservation, WorkflowView } from "./types";

const stage = { number: 10, name: "seed" };
const laterStage = { number: 20, name: "finish" };

class ControlledEventSource extends EventTarget {
  static instances: ControlledEventSource[] = [];
  constructor(readonly url: string, _init?: EventSourceInit) {
    super();
    ControlledEventSource.instances.push(this);
  }
  close() {}
  snapshot(value: WorkflowView) {
    this.dispatchEvent(new MessageEvent("snapshot", { data: JSON.stringify(value) }));
  }
}

function result(overrides: Partial<RoleObservation> = {}): RoleObservation {
  return {
    stage,
    role: "up",
    state: "succeeded",
    exit_code: 0,
    message: null,
    stdout: "",
    stderr: "",
    ...overrides,
  };
}

function observation(roleResults: RoleObservation[] = [result()], state: MovementObservation["state"] = "complete"): MovementObservation {
  return {
    direction: "up",
    target_stage: 10,
    state,
    active_role: null,
    role_results: roleResults,
    failure: null,
    verification_choices: null,
  };
}

function workflow(id: string, overrides: Partial<WorkflowView> = {}): WorkflowView {
  return {
    workspace_name: "preview-fixture",
    workflow: { id, name: id },
    current_status: "available",
    status_issue: null,
    checkpoint: {
      accepted_stage: stage,
      pending_transition: null,
      state: { completed_stage_count: 1, uuid: "fixture-run", pending: null },
    },
    movement_choices: [{ direction: "up", target_stage: 20 }, { direction: "down", target_stage: 0 }],
    stages: [
      { number: 10, name: "seed", state: "accepted", is_accepted_checkpoint: true, definitions: [{ role: "up", path: "stages/010-seed/up" }] },
      { number: 20, name: "finish", state: "future", is_accepted_checkpoint: false, definitions: [{ role: "up", path: "stages/020-finish/up" }] },
    ],
    movement_busy: false,
    observation: observation(),
    ...overrides,
  };
}

function workspaceFor(ids: string[]): WorkspaceView {
  return { name: "preview-fixture", workflows: ids.map((id) => ({ id, name: id })) };
}

function installWorkbench(snapshots: Record<string, WorkflowView>) {
  ControlledEventSource.instances = [];
  const fetchMock = vi.fn((input: RequestInfo | URL, _init?: RequestInit) => {
    const url = String(input);
    if (url === "/api/workspace") return Promise.resolve(Response.json(workspaceFor(Object.keys(snapshots))));
    const stageNumber = url.match(/\/stages\/(\d+)$/)?.[1];
    if (stageNumber) return Promise.resolve(Response.json({ stage: Number(stageNumber) === 20 ? laterStage : stage, definitions: [] }));
    return Promise.resolve(Response.json({}, { status: 404 }));
  });
  vi.stubGlobal("fetch", fetchMock);
  vi.stubGlobal("EventSource", ControlledEventSource);
  render(<App />);
  return fetchMock;
}

async function emitSnapshot(id: string, snapshot: WorkflowView) {
  await waitFor(() => expect(ControlledEventSource.instances.some((source) => source.url.includes("/api/workflows/" + id + "/events"))).toBe(true));
  const source = ControlledEventSource.instances.find((item) => item.url.includes("/api/workflows/" + id + "/events"));
  if (!source) throw new Error("No event source for " + id);
  act(() => source.snapshot(snapshot));
  await screen.findByRole("heading", { name: "Stage inspector" });
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  ControlledEventSource.instances = [];
});

describe("workflow browser adapter", () => {
  it("lists directory identities without claiming every workflow is ready", async () => {
    const healthy = workflow("healthy");
    const unprepared = workflow("unprepared", { current_status: "unavailable", status_issue: "no such table: checkpoint", checkpoint: null, movement_choices: [], stages: [], observation: null });
    installWorkbench({ healthy, unprepared });
    expect(await screen.findByRole("button", { name: /healthy/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /unprepared/ })).toBeInTheDocument();

    await userEvent.setup().click(screen.getByRole("button", { name: /unprepared/ }));
    await emitSnapshot("unprepared", unprepared);
    expect(screen.getByRole("alert")).toHaveTextContent("no such table: checkpoint");
  });

  it("keeps the last observed role output readable while current status is unavailable", async () => {
    const snapshot = workflow("workflow-a", {
      current_status: "unavailable",
      status_issue: "unable to open database file",
      checkpoint: null,
      movement_choices: [],
      stages: [],
      observation: observation([
        result({ stdout: "mutation out", stderr: "mutation err" }),
        result({ role: "verify-up", stdout: "verifier out", stderr: "verifier err" }),
      ]),
    });
    installWorkbench({ "workflow-a": snapshot });
    await emitSnapshot("workflow-a", snapshot);

    expect(screen.getByRole("alert")).toHaveTextContent("unable to open database file");
    expect(screen.getAllByLabelText("stdout").map((element) => element.textContent)).toEqual(["mutation out", "verifier out"]);
    expect(screen.getAllByLabelText("stderr").map((element) => element.textContent)).toEqual(["mutation err", "verifier err"]);
    expect(screen.queryByText("Select a stage")).not.toBeInTheDocument();
    expect(screen.queryByText("CURRENT CHECKPOINT")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Workflow unavailable/ })).toBeDisabled();
    expect(screen.queryByRole("button", { name: /Advance|Back out/ })).not.toBeInTheDocument();
  });

  it("renders buffered stdout and stderr as inert text from the selected workflow snapshot", async () => {
    const snapshot = workflow("workflow-a", {
      observation: observation([result({ stdout: "<script>window.fixture=true</script>", stderr: "separate error text" })]),
    });
    const fetchMock = installWorkbench({ "workflow-a": snapshot });
    await emitSnapshot("workflow-a", snapshot);

    const stdout = screen.getByLabelText("stdout");
    const stderr = screen.getByLabelText("stderr");
    expect(stdout).toHaveTextContent("<script>window.fixture=true</script>");
    expect(stdout.querySelector("script")).toBeNull();
    expect(stderr).toHaveTextContent("separate error text");
    expect(fetchMock.mock.calls.some(([url]) => String(url).includes("/outputs/"))).toBe(false);
  });

  it("keeps an accepted checkpoint separate from unrecorded role results", async () => {
    const snapshot = workflow("workflow-a", {
      stages: [
        {
          number: 10,
          name: "seed",
          state: "accepted",
          is_accepted_checkpoint: true,
          definitions: [
            { role: "up", path: "stages/010-seed/up" },
            { role: "down", path: "stages/010-seed/down" },
            { role: "verify-up", path: "stages/010-seed/verify-up" },
            { role: "verify-down", path: "stages/010-seed/verify-down" },
          ],
        },
      ],
      observation: observation([result({ role: "verify-up" })]),
    });
    installWorkbench({ "workflow-a": snapshot });
    await emitSnapshot("workflow-a", snapshot);

    expect(screen.getByText("Applied", { selector: ".status-pill" })).toBeInTheDocument();
    const roleStatus = (role: string) => {
      const label = screen.getByText(role, { selector: ".role-row-copy b" });
      return label.closest(".role-row")?.querySelector(".role-status")?.textContent;
    };
    expect(roleStatus("up")).toBe("Applied · history unavailable");
    expect(roleStatus("verify-up")).toBe("Process OK");
    expect(roleStatus("down")).toBe("No recorded result");
    expect(roleStatus("verify-down")).toBe("No recorded result");
  });

  it("lets stage selection inspect a future definition without submitting movement", async () => {
    const user = userEvent.setup();
    const snapshot = workflow("workflow-a");
    const fetchMock = installWorkbench({ "workflow-a": snapshot });
    await emitSnapshot("workflow-a", snapshot);

    await user.click(screen.getByRole("button", { name: /STAGE 020.*finish/i }));
    expect(await screen.findByRole("heading", { name: "finish" })).toBeInTheDocument();
    expect(fetchMock.mock.calls.some(([url]) => String(url).includes("/stages/20"))).toBe(true);
    expect(fetchMock.mock.calls.some(([, init]) => (init as RequestInit | undefined)?.method === "POST")).toBe(false);
  });

  it("distinguishes output still buffered during a role from an executable launch error", async () => {
    const snapshot = workflow("workflow-a", {
      observation: observation([result({ state: "in_progress", exit_code: null, stdout: null, stderr: null })], "running"),
    });
    installWorkbench({ "workflow-a": snapshot });
    await emitSnapshot("workflow-a", snapshot);
    expect(screen.getByText("Output will be available when the role returns.")).toBeInTheDocument();

    const failed = workflow("workflow-a", {
      observation: observation([result({ state: "launch_failed", exit_code: null, message: "permission denied by fixture", stdout: null, stderr: null })], "stopped"),
    });
    await emitSnapshot("workflow-a", failed);
    expect(screen.getAllByText("permission denied by fixture").length).toBeGreaterThan(0);
    expect(screen.queryByText("Output will be available when the role returns.")).not.toBeInTheDocument();
  });
});
