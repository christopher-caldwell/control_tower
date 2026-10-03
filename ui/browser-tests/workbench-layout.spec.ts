import { expect, test, type Page } from "@playwright/test";
import type { DefinitionView, ProjectView, WorkspaceView } from "../src/types";

const checkpoint = {
  accepted_stage: { number: 10, name: "seed" },
  pending_transition: null,
  workflow_started: true,
  state: { completed_stage_count: 1, uuid: "fixture-run", pending: null },
};
const workspace: WorkspaceView = {
  project_name: "browser-fixture",
  workspace: { id: "fixture", name: "fixture" },
  current_status: "available",
  status_issue: null,
  movement_busy: false,
  observation: null,
  checkpoint,
  movement_choices: [{ direction: "up", target_stage: 200 }, { direction: "down", target_stage: 0 }],
  selected_stage_number: 10,
  stages: [
    { number: 10, name: "seed", state: "accepted", is_accepted_checkpoint: true, definitions: [{ role: "up", path: "stages/010-seed/up" }] },
    { number: 200, name: "finish", state: "future", is_accepted_checkpoint: false, definitions: [{ role: "up", path: "stages/200-finish/up" }] },
  ],
};
const project: ProjectView = { name: "browser-fixture", workspaces: [{ id: "fixture", name: "fixture" }] };
const definition: DefinitionView = {
  stage: { number: 10, name: "seed" },
  definitions: [{ role: "up", path: "stages/010-seed/up", contents: "#!/bin/sh\nexit 0", issue: null }],
};

type FixtureApi = { movementRequests: () => number };
async function installFixtures(page: Page, failMovement = false, onMovement?: (body: unknown) => void): Promise<FixtureApi> {
  let movementCount = 0;
  await page.route("**/api/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path === "/api/project") return route.fulfill({ json: project });
    if (/\/api\/workspaces\/fixture\/stages\/\d+$/.test(path)) return route.fulfill({ json: definition });
    if (path === "/api/workspaces/fixture/movements") {
      movementCount += 1;
      onMovement?.(route.request().postDataJSON());
      if (failMovement) return route.abort();
      return route.fulfill({ status: 204 });
    }
    return route.fulfill({ status: 404, body: "not found" });
  });
  await page.addInitScript((initial: WorkspaceView) => {
    class FixtureEventSource extends EventTarget {
      onopen: ((event: Event) => void) | null = null;
      onerror: ((event: Event) => void) | null = null;
      constructor(readonly url: string) {
        super();
        queueMicrotask(() => {
          this.onopen?.(new Event("open"));
          this.dispatchEvent(new MessageEvent("snapshot", { data: JSON.stringify(initial) }));
        });
      }
      close() {}
    }
    Object.defineProperty(window, "EventSource", { configurable: true, value: FixtureEventSource });
  }, workspace);
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "seed" })).toBeVisible();
  return { movementRequests: () => movementCount };
}

test("the desktop inspector stays in the right rail and can be collapsed and reopened", async ({ page }) => {
  await installFixtures(page);
  await expect(page.getByRole("complementary", { name: "Selected stage inspector" })).toBeVisible();
  await page.getByRole("button", { name: "Collapse stage inspector" }).click();
  await page.getByRole("button", { name: "Expand stage inspector" }).click();
  await expect(page.getByRole("complementary", { name: "Selected stage inspector" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "seed" })).toBeVisible();
});

test("stage selection inspects a future definition without submitting movement", async ({ page }) => {
  const api = await installFixtures(page);
  const futureStage = page.locator(".stage-card").filter({ hasText: "finish" });
  await futureStage.click();
  await expect(page.getByRole("heading", { name: "finish" })).toBeVisible();
  await expect(futureStage).toHaveAttribute("aria-pressed", "true");
  expect(api.movementRequests()).toBe(0);
});

test("the action dock submits an Application choice with its displayed checkpoint", async ({ page }) => {
  let submitted: unknown;
  await installFixtures(page, false, (body) => { submitted = body; });
  await page.getByRole("button", { name: /Advance to Stage 200/ }).click();
  await expect.poll(() => submitted).toEqual({ direction: "up", target_stage: 200, expected_checkpoint: checkpoint.state });
});

test("a lost movement response is reported without automatic resubmission", async ({ page }) => {
  const api = await installFixtures(page, true);
  await page.getByRole("button", { name: /Advance to Stage 200/ }).click();
  await expect(page.getByText(/Movement response was not confirmed/)).toBeVisible();
  await page.waitForTimeout(250);
  expect(api.movementRequests()).toBe(1);
});
