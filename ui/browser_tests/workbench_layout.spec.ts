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
