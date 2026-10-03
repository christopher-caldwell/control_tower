import { expect, test, type Page } from "@playwright/test";
import type { DefinitionView, ProjectView, WorkspaceView } from "../src/types";

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

async function installApiFixtures(page: Page) {
  await page.route("**/api/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path === "/api/project") return route.fulfill({ json: project });
    if (path === "/api/workspaces/fixture") return route.fulfill({ json: workspace });
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
