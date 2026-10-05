import { expect, type Page, test } from '@playwright/test'

import type { DefinitionView, WorkflowView, WorkspaceView } from '../src/api/types'

const checkpoint = {
  accepted_stage: { number: 10, name: 'seed' },
  pending_transition: null,
  state: { completed_stage_count: 1, uuid: 'fixture-run', pending: null },
}
const workflow: WorkflowView = {
  workspace_name: 'browser-fixture',
  workflow: { id: 'fixture', name: 'fixture' },
  current_status: 'available',
  status_issue: null,
  movement_busy: false,
  observation: null,
  checkpoint,
  movement_choices: [
    { direction: 'up', target_stage: 200 },
    { direction: 'down', target_stage: 0 },
  ],
  stages: [
    {
      number: 10,
      name: 'seed',
      state: 'accepted',
      is_accepted_checkpoint: true,
      definitions: [{ role: 'up', path: 'stages/010-seed/up' }],
    },
    {
      number: 200,
      name: 'finish',
      state: 'future',
      is_accepted_checkpoint: false,
      definitions: [{ role: 'up', path: 'stages/200-finish/up' }],
    },
    {
      number: 500,
      name: 'verify',
      state: 'future',
      is_accepted_checkpoint: false,
      definitions: [{ role: 'up', path: 'stages/500-verify/up' }],
    },
  ],
}
const workspace: WorkspaceView = { name: 'browser-fixture', workflows: [{ id: 'fixture', name: 'fixture' }] }
const definition: DefinitionView = {
  stage: { number: 10, name: 'seed' },
  definitions: [{ role: 'up', path: 'stages/010-seed/up', contents: '#!/bin/sh\nexit 0', issue: null }],
}

type FixtureApi = { movementRequests: () => number }
async function installFixtures(
  page: Page,
  failMovement = false,
  onMovement?: (body: unknown) => void,
  initial: WorkflowView = workflow,
  inventory: WorkspaceView = workspace,
): Promise<FixtureApi> {
  let movementCount = 0
  await page.route(
    (url) => url.pathname.startsWith('/api/'),
    async (route) => {
      const path = new URL(route.request().url()).pathname
      if (path === '/api/workspace') return route.fulfill({ json: inventory })
      if (/\/api\/workflows\/fixture\/stages\/\d+$/.test(path)) return route.fulfill({ json: definition })
      if (path === '/api/workflows/fixture/movements') {
        movementCount += 1
        onMovement?.(route.request().postDataJSON())
        if (failMovement) return route.abort()
        return route.fulfill({ status: 204 })
      }
      return route.fulfill({ status: 404, body: 'not found' })
    },
  )
  await page.addInitScript((initial: WorkflowView) => {
    class FixtureEventSource extends EventTarget {
      onopen: ((event: Event) => void) | null = null
      onerror: ((event: Event) => void) | null = null
      constructor(readonly url: string) {
        super()
        queueMicrotask(() => {
          this.onopen?.(new Event('open'))
          this.dispatchEvent(new MessageEvent('snapshot', { data: JSON.stringify(initial) }))
        })
      }
      close() {}
    }
    Object.defineProperty(window, 'EventSource', { configurable: true, value: FixtureEventSource })
  }, initial)
  await page.goto('/')
  await expect(page.getByRole('heading', { name: 'Stage inspector' })).toBeVisible()
  if (inventory.workflows.length) await expect(page.getByRole('button', { name: 'Refresh workflow' })).toHaveCount(0)
  return { movementRequests: () => movementCount }
}

test('both desktop rails can be collapsed and reopened', async ({ page }) => {
  await installFixtures(page)
  await page.getByRole('button', { name: 'Collapse workflow rail' }).click()
  await page.getByRole('button', { name: 'Expand workflow rail' }).click()
  await expect(page.getByRole('complementary', { name: 'Selected stage inspector' })).toBeVisible()
  const content = page.getByRole('region', { name: 'Stage inspector content' })
  const originalContent = await content.elementHandle()
  await page.getByRole('button', { name: 'Mutation', exact: true }).click()
  await expect(page.getByRole('button', { name: 'Mutation', exact: true })).toHaveAttribute('aria-expanded', 'false')
  await expect(page.getByRole('region', { name: 'Mutation', exact: true, includeHidden: true })).toHaveCSS(
    'height',
    '0px',
  )
  await content.evaluate((element) => {
    element.scrollTop = 80
  })
  const scrollTop = await content.evaluate((element) => element.scrollTop)
  await page.getByRole('button', { name: 'Collapse stage inspector' }).click()
  expect(await originalContent?.evaluate((element) => element.isConnected)).toBe(true)
  await page.getByRole('button', { name: 'Expand stage inspector' }).click()
  await expect(page.getByRole('button', { name: 'Mutation', exact: true })).toHaveAttribute('aria-expanded', 'false')
  expect(await content.evaluate((element) => element.scrollTop)).toBe(scrollTop)
  await expect(page.getByRole('complementary', { name: 'Selected stage inspector' })).toBeVisible()
  await expect(page.getByRole('heading', { name: 'Stage inspector' })).toBeVisible()
  await expect(page.getByRole('separator', { name: 'Resize stage inspector' })).toHaveCount(0)
  expect(await page.evaluate(() => localStorage.getItem('control-tower-workbench-layout-v1'))).toBeNull()
})

test('stage selection inspects a future definition without submitting movement', async ({ page }) => {
  const api = await installFixtures(page)
  const futureStage = page.getByRole('button', { name: /Not applied finish/ })
  await futureStage.click()
  await expect(page.getByRole('heading', { name: 'finish' })).toBeVisible()
  await expect(futureStage).toHaveAttribute('aria-pressed', 'true')
  await expect(page.getByRole('button', { name: /Run to Stage/ })).toHaveCount(0)
  expect(api.movementRequests()).toBe(0)
})

test('the action dock submits an Application choice with its displayed checkpoint', async ({ page }) => {
  let submitted: unknown
  await installFixtures(page, false, (body) => {
    submitted = body
  })
  await page.getByRole('button', { name: 'Run next' }).click()
  await expect
    .poll(() => submitted)
    .toEqual({ direction: 'up', target_stage: 200, expected_checkpoint: checkpoint.state })
})

test('Run to targets the selected farther stage in one movement request', async ({ page }) => {
  let submitted: unknown
  const api = await installFixtures(page, false, (body) => {
    submitted = body
  })
  await page.getByRole('button', { name: /Not applied verify/ }).click()
  const runTo = page.getByRole('button', { name: /Run to Stage 500 · verify/ })
  await expect(runTo).toBeVisible()
  expect(api.movementRequests()).toBe(0)
  await runTo.click()
  await expect
    .poll(() => submitted)
    .toEqual({ direction: 'up', target_stage: 500, expected_checkpoint: checkpoint.state })
  expect(api.movementRequests()).toBe(1)
})

test('Run all targets the final stage with the existing checkpoint contract', async ({ page }) => {
  let submitted: unknown
  const api = await installFixtures(page, false, (body) => {
    submitted = body
  })
  await page.getByRole('button', { name: 'Run all' }).click()
  await expect
    .poll(() => submitted)
    .toEqual({ direction: 'up', target_stage: 500, expected_checkpoint: checkpoint.state })
  expect(api.movementRequests()).toBe(1)
})

test('a lost movement response is reported without automatic resubmission', async ({ page }) => {
  const api = await installFixtures(page, true)
  await page.getByRole('button', { name: 'Run next' }).click()
  await expect(page.getByText(/Movement response was not confirmed/)).toBeVisible()
  await page.waitForTimeout(250)
  expect(api.movementRequests()).toBe(1)
})

const visualStates: Record<string, WorkflowView> = {
  populated: workflow,
  running: {
    ...workflow,
    movement_busy: true,
    observation: {
      direction: 'up',
      target_stage: 200,
      state: 'running',
      active_role: { stage: { number: 200, name: 'finish' }, role: 'up' },
      role_results: [
        {
          stage: { number: 200, name: 'finish' },
          role: 'up',
          state: 'in_progress',
          exit_code: null,
          message: null,
          stdout: null,
          stderr: null,
        },
      ],
      failure: null,
      verification_choices: null,
    },
  },
  pending: {
    ...workflow,
    checkpoint: {
      ...checkpoint,
      pending_transition: { direction: 'up', stage: { number: 200, name: 'finish' } },
      state: { ...checkpoint.state, pending: { direction: 'up', stage_index: 1 } },
    },
    stages: workflow.stages.map((stage) => (stage.number === 200 ? { ...stage, state: 'pending' } : stage)),
    observation: {
      direction: 'up',
      target_stage: 200,
      state: 'stopped',
      active_role: null,
      role_results: [
        {
          stage: { number: 200, name: 'finish' },
          role: 'verify-up',
          state: 'failed',
          exit_code: 1,
          message: 'Expected fixture was not found.',
          stdout: 'Verification inspected the fixture.',
          stderr: 'Expected fixture was not found.',
        },
      ],
      failure: {
        kind: 'verification_failed',
        message: 'Expected fixture was not found.',
        stage: { number: 200, name: 'finish' },
        role: 'verify-up',
      },
      verification_choices: {
        retry: { direction: 'up', target_stage: 200 },
        reverse: { direction: 'down', target_stage: 10 },
      },
    },
  },
  error: {
    ...workflow,
    current_status: 'unavailable',
    status_issue: 'Unable to open the checkpoint database. Prepare the workflow before running it.',
    checkpoint: null,
    movement_choices: [],
    stages: [],
  },
}

for (const width of [1280, 1600]) {
  for (const [state, initial] of Object.entries(visualStates)) {
    test(`desktop ${state} view at ${width}`, async ({ page }) => {
      await page.setViewportSize({ width, height: width === 1280 ? 820 : 1000 })
      await installFixtures(page, false, undefined, initial)
      const rails = page.getByRole('complementary')
      await expect(rails).toHaveCount(2)
      const left = await rails.nth(0).boundingBox(),
        right = await rails.nth(1).boundingBox()
      expect(left?.width).toBe(252)
      expect(right?.width).toBe(382)
      await expect(page.getByRole('button', { name: 'Collapse stage inspector' })).toBeVisible()
      if (state === 'populated') {
        await expect(page.getByRole('button', { name: /Not applied finish/ })).toContainText('200')
        await expect(page.getByRole('button', { name: 'Run next' })).toBeVisible()
      }
      if (state === 'running') {
        await expect(page.getByRole('button', { name: 'Run next' })).toBeDisabled()
        await expect(page.getByText('Movement active · up', { exact: true })).toBeVisible()
      }
      if (state === 'pending')
        await expect(page.getByRole('button', { name: 'Retry verify-up for Stage 200' })).toBeVisible()
      await page.screenshot({ path: `../output/playwright/${state}-${width}.png` })
    })
  }
  test(`desktop empty view at ${width}`, async ({ page }) => {
    await page.setViewportSize({ width, height: width === 1280 ? 820 : 1000 })
    await installFixtures(page, false, undefined, workflow, { ...workspace, workflows: [] })
    await expect(page.getByRole('heading', { name: 'No workflows found' })).toBeVisible()
    await page.screenshot({ path: `../output/playwright/empty-${width}.png` })
  })
  test(`desktop loading view at ${width}`, async ({ page }) => {
    await page.setViewportSize({ width, height: width === 1280 ? 820 : 1000 })
    await page.route('**/api/workspace', (route) => route.fulfill({ json: workspace }))
    await page.addInitScript(() => {
      class ConnectingSource extends EventTarget {
        close() {}
      }
      Object.defineProperty(window, 'EventSource', { value: ConnectingSource })
    })
    await page.goto('/')
    await expect(page.getByText('Waiting for a fresh workflow snapshot…')).toBeVisible()
    await expect(page.getByRole('button', { name: 'Connecting to workflow' })).toBeDisabled()
    await page.screenshot({ path: `../output/playwright/loading-${width}.png` })
  })
}

test('keyboard inspection keeps movement read-only and collapse resets on reload', async ({ page }) => {
  const api = await installFixtures(page)
  const future = page.getByRole('button', { name: /Not applied finish/ })
  await future.focus()
  await page.keyboard.press('Enter')
  await expect(page.getByRole('heading', { name: 'finish' })).toBeVisible()
  expect(api.movementRequests()).toBe(0)
  await page.getByRole('button', { name: 'Collapse workflow rail' }).click()
  await page.getByRole('button', { name: 'Collapse stage inspector' }).click()
  await page.reload()
  await expect(page.getByRole('button', { name: 'Collapse workflow rail' })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Collapse stage inspector' })).toBeVisible()
})

test('long names and output preserve pane widths and the action dock', async ({ page }) => {
  const longName = 'a-very-long-stage-name-with-many-details-'.repeat(5)
  const initial: WorkflowView = {
    ...workflow,
    workflow: { id: 'fixture', name: longName },
    stages: workflow.stages.map((stage) => ({ ...stage, name: longName })),
    observation: {
      direction: 'up',
      target_stage: 10,
      state: 'complete',
      active_role: null,
      role_results: [
        {
          stage: { number: 10, name: longName },
          role: 'up',
          state: 'succeeded',
          exit_code: 0,
          message: null,
          stdout: '<script>window.unsafe=true</script>\n' + 'long-output-'.repeat(1000),
          stderr: 'a separate error stream',
        },
      ],
      failure: null,
      verification_choices: null,
    },
  }
  await installFixtures(page, false, undefined, initial, { ...workspace, name: longName })
  await expect(page.getByLabel('stdout')).toContainText('<script>window.unsafe=true</script>')
  expect(await page.getByLabel('stdout').locator('script').count()).toBe(0)
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(1280)
  const inspector = page.getByRole('complementary', { name: 'Selected stage inspector' })
  expect((await inspector.boundingBox())?.width).toBe(382)
  await page
    .getByRole('button', { name: /Not applied/ })
    .last()
    .click()
  await expect(page.getByRole('button', { name: /Run to Stage 500/ })).toBeVisible()
  const runTo = await page.getByRole('button', { name: /Run to Stage 500/ }).boundingBox()
  expect(runTo!.x + runTo!.width).toBeLessThanOrEqual(898)
  await expect(page.getByRole('button', { name: 'Run next' })).toBeVisible()
  await page.screenshot({ path: '../output/playwright/long-content-1280.png' })
})
