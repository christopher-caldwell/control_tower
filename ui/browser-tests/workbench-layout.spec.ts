import { expect, test, type Page } from "@playwright/test";
import type { DefinitionView, MovementObservation, ProjectView, RuntimeSnapshot, WorkspaceView } from "../src/types";

const project: ProjectView = {
  name: "layout-project",
  workspace_root: "workspaces/",
  discovery_error: null,
  workspaces: [{
    id: "fixture",
    name: "fixture",
    available: true,
    issue: null,
    stage_count: 2,
    accepted_stage: { number: 10, name: "seed" },
    pending_transition: { direction: "down", stage: { number: 200, name: "finish" } },
  }],
};

const workspace: WorkspaceView = {
  project_name: "layout-project",
  workspace: { id: "fixture", name: "fixture" },
  server_instance_id: "layout-server",
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
      definitions: [{ role: "up", path: "stages/010-seed/up" }],
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
  definitions: workspace.stages[1].definitions.map((item) => ({
    ...item,
    contents: "#!/bin/sh\nexit 0",
    truncated: false,
    issue: null,
  })),
};

async function installApiFixtures(page: Page, projectFixture = project, workspaceFixture = workspace) {
  await page.route("**/api/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path === "/api/project") return route.fulfill({ json: projectFixture });
    if (path === "/api/workspaces/fixture") return route.fulfill({ json: workspaceFixture });
    if (path === "/api/workspaces/fixture/stages/200") return route.fulfill({ json: definition });
    return route.fulfill({ status: 404, body: "not found" });
  });
}

async function layout(page: Page) {
  return page.evaluate(() => {
    const rect = (selector: string) => {
      const element = document.querySelector(selector);
      if (!element) throw new Error(`Missing layout element: ${selector}`);
      const { x, width, right } = element.getBoundingClientRect();
      return { x, width, right };
    };
    return {
      viewportWidth: window.innerWidth,
      documentWidth: document.documentElement.scrollWidth,
      stage: rect(".stage-column"),
      splitter: rect(".inspector-resize"),
      inspector: rect(".inspector-rail"),
    };
  });
}

test.beforeEach(async ({ page }) => {
  await installApiFixtures(page);
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "finish" })).toBeVisible();
});

test("collapse and reopen keep the inspector in its explicit right-rail column", async ({ page }) => {
  const inspector = page.locator(".inspector-rail");
  await expect.poll(async () => Math.round((await layout(page)).inspector.width)).toBe(382);

  await page.getByRole("button", { name: "Collapse stage inspector" }).click();
  await expect.poll(async () => Math.round((await layout(page)).inspector.width)).toBe(54);
  let current = await layout(page);
  expect(current.inspector.right).toBe(current.viewportWidth);
  expect(current.documentWidth).toBe(current.viewportWidth);

  await page.getByRole("button", { name: "Expand stage inspector" }).click();
  await expect.poll(async () => Math.round((await layout(page)).inspector.width)).toBe(382);
  current = await layout(page);
  expect(current.inspector.right).toBe(current.viewportWidth);
  expect(current.stage.width).toBeGreaterThanOrEqual(520);
  await expect(inspector).toBeVisible();
});

test("resizing at minimum desktop width preserves the central narrative and viewport fit", async ({ page }) => {
  const handle = page.getByRole("separator", { name: "Resize stage inspector" });
  const handleBox = await handle.boundingBox();
  expect(handleBox).not.toBeNull();
  await page.mouse.move(handleBox!.x + handleBox!.width / 2, handleBox!.y + handleBox!.height / 2);
  await page.mouse.down();
  await page.mouse.move(0, handleBox!.y + handleBox!.height / 2);
  await page.mouse.up();

  await expect.poll(async () => Math.round((await layout(page)).inspector.width)).toBe(394);
  await expect(handle).toHaveAttribute("aria-valuemax", "394");
  await expect(handle).toHaveAttribute("aria-valuenow", "394");
  let current = await layout(page);
  expect(current.viewportWidth).toBe(1180);
  expect(current.stage.width).toBeGreaterThanOrEqual(520);
  expect(current.inspector.right).toBe(current.viewportWidth);
  expect(current.documentWidth).toBe(current.viewportWidth);

  await page.getByRole("button", { name: "Collapse stage inspector" }).click();
  await expect.poll(async () => Math.round((await layout(page)).inspector.width)).toBe(54);
  await page.getByRole("button", { name: "Expand stage inspector" }).click();
  await expect.poll(async () => Math.round((await layout(page)).inspector.width)).toBe(394);
  current = await layout(page);
  expect(current.inspector.right).toBe(current.viewportWidth);
  expect(current.documentWidth).toBe(current.viewportWidth);
});

test("clamps a saved inspector width from a wider window at the minimum viewport", async ({ page }) => {
  await page.evaluate(() => localStorage.setItem("control-tower-workbench-layout-v1", JSON.stringify({
    leftCollapsed: false,
    rightCollapsed: false,
    rightWidth: 414,
  })));
  await page.reload();
  await expect(page.getByRole("heading", { name: "finish" })).toBeVisible();

  const handle = page.getByRole("separator", { name: "Resize stage inspector" });
  await expect.poll(async () => Math.round((await layout(page)).inspector.width)).toBe(394);
  await expect(handle).toHaveAttribute("aria-valuenow", "394");
  const current = await layout(page);
  expect(current.stage.width).toBeGreaterThanOrEqual(520);
  expect(current.inspector.right).toBe(current.viewportWidth);
  expect(current.documentWidth).toBe(current.viewportWidth);
});

test("an observing browser tab reconciles another tab's completion into its rail and movement controls", async ({ context, page }) => {
  const baseline = { accepted_stage: null, pending_transition: null, workflow_started: false };
  const baselineProject: ProjectView = {
    ...project,
    workspaces: project.workspaces.map((item) => item.id === "fixture"
      ? { ...item, accepted_stage: null, pending_transition: null }
      : item),
  };
  const baselineWorkspace: WorkspaceView = {
    ...workspace,
    checkpoint: baseline,
    observation: null,
    selected_stage_number: 10,
    stages: workspace.stages.map((stage) => ({
      ...stage,
      state: "future",
      is_accepted_checkpoint: false,
      definitions: stage.number === 10 ? [...stage.definitions, { role: "down", path: "stages/010-seed/down" }] : stage.definitions,
    })),
  };

  await page.unroute("**/api/**");
  await installApiFixtures(page, baselineProject, baselineWorkspace);
  await context.addInitScript(() => {
    class ControlledEventSource extends EventTarget {
      static instances: ControlledEventSource[] = [];
      readonly url: string;
      onopen: ((event: Event) => void) | null = null;
      onerror: ((event: Event) => void) | null = null;
      constructor(url: string) {
        super();
        this.url = url;
        ControlledEventSource.instances.push(this);
        queueMicrotask(() => this.onopen?.(new Event("open")));
      }
      close() {}
      emit(name: string, value: unknown) {
        this.dispatchEvent(new MessageEvent(name, { data: JSON.stringify(value) }));
      }
    }
    Object.defineProperty(window, "EventSource", { configurable: true, value: ControlledEventSource });
    Object.defineProperty(window, "__emitControlTowerEvent", {
      configurable: true,
      value: (name: string, value: RuntimeSnapshot) => ControlledEventSource.instances
        .filter((source) => source.url.includes(`/workspaces/${value.workspace_id}/events`))
        .forEach((source) => source.emit(name, value)),
    });
  });
  await page.reload();
  await expect(page.getByRole("button", { name: /fixture At baseline/ })).toBeVisible();

  const otherTab = await context.newPage();
  await installApiFixtures(otherTab, baselineProject, baselineWorkspace);
  await otherTab.goto("/");
  await expect(otherTab.getByRole("button", { name: /fixture At baseline/ })).toBeVisible();

  const checkpoint = { accepted_stage: { number: 10, name: "seed" }, pending_transition: null, workflow_started: true };
  const observation: MovementObservation = {
    workspace_id: "fixture", operation_id: "other-tab-complete", server_instance_id: "layout-server", revision: 10,
    direction: "up", target_stage: 10, state: "complete", active_role: null,
    role_results: [], omitted_role_results: 0, outputs_evicted: 0,
    confirmed_checkpoint: checkpoint, attempted_checkpoint: null, failure: null, verification_choices: null,
  };
  const snapshot: RuntimeSnapshot = {
    workspace_id: "fixture", server_instance_id: "layout-server", revision: 10, movement_busy: false,
    checkpoint, observation,
  };
  await page.evaluate((value) => {
    (window as unknown as Window & { __emitControlTowerEvent: (name: string, snapshot: RuntimeSnapshot) => void })
      .__emitControlTowerEvent("resync", value);
  }, snapshot);
  await otherTab.evaluate((value) => {
    (window as unknown as Window & { __emitControlTowerEvent: (name: string, snapshot: RuntimeSnapshot) => void })
      .__emitControlTowerEvent("resync", value);
  }, snapshot);

  await expect(page.getByText("Movement completed")).toBeVisible();
  await expect(page.getByRole("button", { name: /Applied · 10/ })).toBeVisible();
  await expect(page.getByText("10 · seed")).toBeVisible();
  await expect(page.getByRole("button", { name: /Back out Stage 10 to baseline/ })).toBeEnabled();
  await expect(otherTab.getByRole("button", { name: /Applied · 10/ })).toBeVisible();
  await expect(otherTab.getByRole("button", { name: /Back out Stage 10 to baseline/ })).toBeEnabled();
});
