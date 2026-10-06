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
const longStageName = 'Seed stage with a long descriptive workflow operation that should wrap cleanly in the inspector'
const longTargetName = 'Long target stage name that remains fully discoverable in the forward movement action dock'
const longOutput = Array.from(
  { length: 300 },
  (_, index) => `output line ${index}: ${'observed role detail '.repeat(4)}`,
).join('\n')
const longDefinition: DefinitionView = {
  ...definition,
  definitions: [{ ...definition.definitions[0], contents: '#!/bin/sh\n' + 'echo long-definition-line\n'.repeat(300) }],
}

const workflowWithLongContent: WorkflowView = {
  ...workflow,
  checkpoint: { ...checkpoint, accepted_stage: { number: 10, name: longStageName } },
  stages: workflow.stages.map((stage) => (stage.number === 10 ? { ...stage, name: longStageName } : stage)),
  observation: {
    direction: 'up',
    target_stage: 10,
    state: 'complete',
    active_role: null,
    role_results: [
      {
        stage: { number: 10, name: longStageName },
        role: 'up',
        state: 'succeeded',
        exit_code: 0,
        message: null,
        stdout: longOutput,
        stderr: longOutput,
      },
    ],
    failure: null,
    verification_choices: null,
  },
}
const workflowWithLongTarget: WorkflowView = {
  ...workflow,
  stages: workflow.stages.map((stage) => (stage.number === 500 ? { ...stage, name: longTargetName } : stage)),
}
const secondWorkflow: WorkflowView = {
  ...workflow,
  workflow: { id: 'second', name: 'Second long workflow name' },
  checkpoint: { ...checkpoint, accepted_stage: { number: 10, name: 'beta checkpoint' } },
  stages: workflow.stages.map((stage) => (stage.number === 10 ? { ...stage, name: 'beta checkpoint' } : stage)),
}
const resizedWorkflowWorkspace: WorkspaceView = {
  name: 'browser-fixture',
  workflows: [
    { id: 'fixture', name: 'Primary workflow' },
    { id: 'second', name: 'Second long workflow name' },
  ],
}

type FixtureApi = {
  movementRequests: () => number
  inventoryRequests: () => number
  workflowReadRequests: () => number
  definitionRequests: () => number
  eventSourceRequests: () => Promise<string[]>
}
const isWorkflowSnapshot = (value: WorkflowView | Record<string, WorkflowView>): value is WorkflowView =>
  'workspace_name' in value && 'workflow' in value && 'current_status' in value

async function installFixtures(
  page: Page,
  failMovement = false,
  onMovement?: (body: unknown) => void,
  initial: WorkflowView | Record<string, WorkflowView> = workflow,
  inventory: WorkspaceView = workspace,
  longContent = false,
): Promise<FixtureApi> {
  let movementCount = 0
  let inventoryCount = 0
  let workflowReadCount = 0
  let definitionCount = 0
  const snapshots: Record<string, WorkflowView> = isWorkflowSnapshot(initial)
    ? { [initial.workflow.id]: initial }
    : initial
  await page.route(
    (url) => url.pathname.startsWith('/api/'),
    async (route) => {
      const path = new URL(route.request().url()).pathname
      if (path === '/api/workspace') {
        inventoryCount += 1
        return route.fulfill({ json: inventory })
      }
      if (/\/api\/workflows\/[^/]+\/stages\/\d+$/.test(path)) {
        definitionCount += 1
        return route.fulfill({ json: longContent ? longDefinition : definition })
      }
      if (/\/api\/workflows\/[^/]+\/movements$/.test(path)) {
        movementCount += 1
        onMovement?.(route.request().postDataJSON())
        if (failMovement) return route.abort()
        return route.fulfill({ status: 204 })
      }
      if (/\/api\/workflows\/[^/]+$/.test(path)) workflowReadCount += 1
      return route.fulfill({ status: 404, body: 'not found' })
    },
  )
  await page.addInitScript((snapshots: Record<string, WorkflowView>) => {
    const eventSourceRequests: string[] = []
    Object.defineProperty(window, '__fixtureEventSourceRequests', { configurable: true, value: eventSourceRequests })
    class FixtureEventSource extends EventTarget {
      onopen: ((event: Event) => void) | null = null
      onerror: ((event: Event) => void) | null = null
      constructor(readonly url: string) {
        eventSourceRequests.push(url)
        super()
        const workflowId = url.match(/\/api\/workflows\/([^/]+)\/events/)?.[1]
        const snapshot = (workflowId && snapshots[workflowId]) || Object.values(snapshots)[0]
        queueMicrotask(() => {
          this.onopen?.(new Event('open'))
          this.dispatchEvent(new MessageEvent('snapshot', { data: JSON.stringify(snapshot) }))
        })
      }
      close() {}
    }
    Object.defineProperty(window, 'EventSource', { configurable: true, value: FixtureEventSource })
  }, snapshots)
  await page.goto('/')
  await expect(page.getByRole('heading', { name: 'Stage inspector' })).toBeVisible()
  if (inventory.workflows.length) await expect(page.getByRole('button', { name: 'Refresh workflow' })).toHaveCount(0)
  return {
    movementRequests: () => movementCount,
    inventoryRequests: () => inventoryCount,
    workflowReadRequests: () => workflowReadCount,
    definitionRequests: () => definitionCount,
    eventSourceRequests: () =>
      page.evaluate(() => {
        const fixtureWindow = window as Window & { __fixtureEventSourceRequests?: string[] }
        return fixtureWindow.__fixtureEventSourceRequests ?? []
      }),
  }
}

test('both desktop rails can be collapsed and reopened', async ({ page }) => {
  await installFixtures(
    page,
    false,
    undefined,
    { fixture: workflowWithLongContent, second: secondWorkflow },
    resizedWorkflowWorkspace,
    true,
  )
  const inspector = page.getByRole('complementary', { name: 'Selected stage inspector' })
  const separator = page.getByRole('separator', { name: 'Resize stage inspector' })
  await separator.focus()
  await separator.press('ArrowLeft')
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(398)
  await page.getByRole('button', { name: 'Second long workflow name' }).click()
  await expect(page.getByRole('heading', { name: 'Second long workflow name' })).toBeVisible()
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(398)
  await page.getByRole('button', { name: 'Primary workflow' }).click()
  await expect(page.getByRole('heading', { name: 'Primary workflow' })).toBeVisible()
  await expect(page.getByRole('heading', { name: longStageName })).toBeVisible()
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(398)
  await page.getByRole('button', { name: 'Collapse workflow rail' }).click()
  await page.getByRole('button', { name: 'Expand workflow rail' }).click()
  await expect(page.getByRole('complementary', { name: 'Selected stage inspector' })).toBeVisible()
  const content = page.getByRole('region', { name: 'Stage inspector content' })
  const originalContent = await content.elementHandle()
  expect(await content.evaluate((element) => element.scrollHeight > element.clientHeight)).toBe(true)
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
  await expect(page.getByRole('separator', { name: 'Resize stage inspector' })).toHaveCount(0)
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(54)
  expect(await originalContent?.evaluate((element) => element.isConnected)).toBe(true)
  await page.getByRole('button', { name: 'Expand stage inspector' }).click()
  await expect(page.getByRole('button', { name: 'Mutation', exact: true })).toHaveAttribute('aria-expanded', 'false')
  expect(await content.evaluate((element) => element.scrollTop)).toBe(scrollTop)
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(398)
  await page.getByRole('button', { name: 'Collapse stage inspector' }).click()
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(54)
  await page.getByText('finish', { exact: true }).click()
  await expect(page.getByRole('button', { name: 'Expand stage inspector' })).toBeVisible()
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(54)
  await page.getByRole('button', { name: 'Expand stage inspector' }).click()
  await expect(page.getByRole('complementary', { name: 'Selected stage inspector' })).toBeVisible()
  await expect(page.getByRole('heading', { name: 'finish' })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Mutation', exact: true })).toHaveAttribute('aria-expanded', 'false')
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(398)
  await expect(page.getByRole('separator', { name: 'Resize stage inspector' })).toHaveCount(1)
  expect(await page.evaluate(() => localStorage.getItem('control-tower-stage-inspector-width-v1'))).toBe('398')
  await page.getByRole('button', { name: 'Collapse stage inspector' }).click()
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(54)
  await page.getByRole('button', { name: 'Collapse workflow rail' }).click()
  await expect(page.getByRole('button', { name: 'Expand workflow rail' })).toBeVisible()
  await expect
    .poll(() =>
      page
        .getByRole('complementary', { name: 'Workspace workflows' })
        .evaluate((element) => element.getBoundingClientRect().width),
    )
    .toBe(60)
  await page.reload()
  await expect(page.getByRole('button', { name: 'Collapse workflow rail' })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Collapse stage inspector' })).toBeVisible()
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(398)
})

test('inspector keeps a stable consuming scrollbar gutter across real content overflow', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 820 })
  await installFixtures(page, false, undefined, workflowWithLongContent, workspace, true)
  const inspector = page.getByRole('complementary', { name: 'Selected stage inspector' })
  const content = page.getByRole('region', { name: 'Stage inspector content' })
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(382)
  await page.addStyleTag({
    content: `
      [aria-label="Stage inspector content"]::-webkit-scrollbar { width: 16px; }
      [aria-label="Stage inspector content"]::-webkit-scrollbar-thumb { background: #888; }
    `,
  })
  const accordionNames = ['Checkpoint state', 'Mutation', 'Verification', 'Captured output', 'Executable definitions']
  for (const name of accordionNames) {
    const control = page.getByRole('button', { name, exact: true })
    if ((await control.getAttribute('aria-expanded')) === 'true') await control.click()
    await expect(control).toHaveAttribute('aria-expanded', 'false')
  }
  await expect.poll(() => content.evaluate((element) => element.scrollHeight <= element.clientHeight)).toBe(true)
  const before = await content.evaluate((element) => {
    const contentElement = element as HTMLElement
    return {
      clientHeight: element.clientHeight,
      clientWidth: element.clientWidth,
      scrollHeight: element.scrollHeight,
      overflowing: element.scrollHeight > element.clientHeight,
      consumingScrollbar: contentElement.offsetWidth - element.clientWidth,
    }
  })
  expect(before.overflowing).toBe(false)
  expect(before.consumingScrollbar).toBeGreaterThanOrEqual(16)
  const mutation = page.getByRole('button', { name: 'Mutation', exact: true })
  const controlRightBefore = await mutation.evaluate((element) => element.getBoundingClientRect().right)
  const expandedForOverflow: string[] = []
  let previousScrollHeight = before.clientHeight
  for (const name of ['Captured output', 'Executable definitions', 'Mutation', 'Verification', 'Checkpoint state']) {
    const control = page.getByRole('button', { name, exact: true })
    await control.click()
    await expect(control).toHaveAttribute('aria-expanded', 'true')
    await expect.poll(() => content.evaluate((element) => element.scrollHeight)).toBeGreaterThan(previousScrollHeight)
    previousScrollHeight = await content.evaluate((element) => element.scrollHeight)
    expandedForOverflow.push(name)
    if (previousScrollHeight > before.clientHeight) break
  }
  expect(expandedForOverflow).toContain('Captured output')
  expect(previousScrollHeight).toBeGreaterThan(before.clientHeight)
  expect(await page.getByLabel('stdout').textContent()).toContain(longOutput.slice(0, 120))
  expect(await page.getByLabel('stderr').textContent()).toContain(longOutput.slice(0, 120))
  const after = await content.evaluate((element) => {
    const contentElement = element as HTMLElement
    return {
      clientHeight: element.clientHeight,
      clientWidth: element.clientWidth,
      scrollHeight: element.scrollHeight,
      overflowing: element.scrollHeight > element.clientHeight,
      consumingScrollbar: contentElement.offsetWidth - element.clientWidth,
    }
  })
  expect(after.overflowing).toBe(true)
  expect(after.clientHeight).toBe(before.clientHeight)
  expect(after.consumingScrollbar).toBeGreaterThanOrEqual(16)
  expect(after.clientWidth).toBe(before.clientWidth)
  expect(await mutation.evaluate((element) => element.getBoundingClientRect().right)).toBe(controlRightBefore)
  await expect(mutation).toBeVisible()
  await mutation.click()
  await expect(mutation).toHaveAttribute('aria-expanded', 'true')
})

test('forward dock emphasis and no-action status use the selected presentation', async ({ page }) => {
  await installFixtures(page)
  await expect(page.getByRole('button', { name: 'Run next' })).toHaveAttribute('data-variant', 'filled')
  await page.getByText('verify', { exact: true }).click()
  await expect(page.getByRole('button', { name: /Run to/ })).toHaveAttribute('data-variant', 'light')
  await expect(page.getByRole('button', { name: 'Run all' })).toHaveAttribute('data-variant', 'light')
})

test('long forward targets stay discoverable through hover and keyboard focus', async ({ page }) => {
  const api = await installFixtures(page, false, undefined, workflowWithLongTarget)
  await page.getByText(longTargetName, { exact: true }).click()
  const label = 'Run to Stage 500 · ' + longTargetName
  const runTo = page.getByRole('button', { name: label })
  await expect(runTo).toBeVisible()
  await expect(runTo).toHaveAttribute('data-variant', 'light')
  await runTo.hover()
  await expect(page.getByRole('tooltip')).toHaveText(label)

  const runNext = page.getByRole('button', { name: 'Run next' })
  await runNext.hover()
  await expect(runNext).toHaveAttribute('data-variant', 'filled')
  const search = page.getByRole('textbox', { name: 'Search workflows' })
  await search.focus()
  for (let step = 0; step < 30; step += 1) {
    if (await runTo.evaluate((element) => document.activeElement === element)) break
    await page.keyboard.press('Tab')
  }
  await expect(runTo).toBeFocused()
  expect(await api.movementRequests()).toBe(0)
})

test('no-action dock text has no button semantics', async ({ page }) => {
  const noMovement = { ...workflow, movement_choices: [] }
  await installFixtures(page, false, undefined, noMovement)
  await expect(page.getByText('No immediate movement available')).toBeVisible()
  await expect(page.getByRole('button', { name: 'No immediate movement available' })).toHaveCount(0)
})

test('inspector width resizes by pointer and keyboard and restores from width-only storage', async ({ page }) => {
  const api = await installFixtures(page, false, undefined, workflowWithLongContent, workspace, true)
  const inspector = page.getByRole('complementary', { name: 'Selected stage inspector' })
  const separator = page.getByRole('separator', { name: 'Resize stage inspector' })
  const inspectorContent = page.getByRole('region', { name: 'Stage inspector content' })
  const initialWidth = await inspector.evaluate((element) => element.getBoundingClientRect().width)
  expect(initialWidth).toBe(382)
  await expect(separator).toHaveAttribute('aria-controls', 'stage-inspector-pane')
  await expect(separator).toHaveAttribute('aria-orientation', 'vertical')
  await expect(separator).toHaveAttribute('aria-valuenow', '382')
  await expect(page.getByRole('heading', { name: 'Stage inspector' })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Collapse stage inspector' })).toBeVisible()
  await expect(page.getByRole('heading', { name: longStageName })).toBeVisible()
  expect(await inspectorContent.evaluate((element) => element.scrollHeight > element.clientHeight)).toBe(true)
  expect(await page.getByLabel('stdout').evaluate((element) => getComputedStyle(element).fontSize)).toBe('12px')
  expect(
    await page
      .getByRole('button', { name: new RegExp(longStageName) })
      .evaluate((element) => getComputedStyle(element).paddingTop),
  ).toBe('0px')
  expect(await api.movementRequests()).toBe(0)

  await separator.focus()
  await expect(separator).toBeFocused()
  await separator.press('ArrowRight')
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(366)
  await expect(separator).toHaveAttribute('aria-valuenow', '366')
  await separator.press('ArrowLeft')
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(382)

  const box = await separator.boundingBox()
  if (!box) throw new Error('Resize separator has no rendered bounds')
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2)
  await page.mouse.down()
  await page.mouse.move(box.x - 30, box.y + box.height / 2)
  await page.mouse.up()
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(416)
  await page.mouse.move(box.x - 80, box.y + box.height / 2)
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(416)

  const currentBox = await separator.boundingBox()
  if (!currentBox) throw new Error('Resize separator has no rendered bounds')
  await page.mouse.move(currentBox.x + currentBox.width / 2, currentBox.y + currentBox.height / 2)
  await page.mouse.down()
  await page.mouse.move(currentBox.x + currentBox.width / 2 + 12, currentBox.y + currentBox.height / 2)
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(404)
  await separator.evaluate((element) =>
    element.dispatchEvent(new PointerEvent('pointercancel', { bubbles: true, pointerId: 1 })),
  )
  await page.mouse.move(currentBox.x + currentBox.width / 2 + 80, currentBox.y + currentBox.height / 2)
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(404)
  await page.mouse.up()
  expect(await page.evaluate(() => getSelection()?.toString() ?? '')).toBe('')

  const narrowerBox = await separator.boundingBox()
  if (!narrowerBox) throw new Error('Resize separator has no rendered bounds')
  await page.mouse.move(narrowerBox.x + narrowerBox.width / 2, narrowerBox.y + narrowerBox.height / 2)
  await page.mouse.down()
  await page.mouse.move(narrowerBox.x + narrowerBox.width / 2 + 10, narrowerBox.y + narrowerBox.height / 2)
  await page.mouse.up()
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(394)
  const expandBox = await separator.boundingBox()
  if (!expandBox) throw new Error('Resize separator has no rendered bounds')
  await page.mouse.move(expandBox.x + expandBox.width / 2, expandBox.y + expandBox.height / 2)
  await page.mouse.down()
  await page.mouse.move(expandBox.x - 500, expandBox.y + expandBox.height / 2)
  await page.mouse.up()
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(508)
  await expect(separator).toHaveAttribute('aria-valuemax', '508')
  await expect(separator).toHaveAttribute('aria-valuenow', '508')
  await separator.press('ArrowLeft')
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(508)
  expect(await page.evaluate(() => localStorage.getItem('control-tower-stage-inspector-width-v1'))).toBe('508')
  expect(await api.movementRequests()).toBe(0)

  await page.getByRole('textbox', { name: 'Search workflows' }).focus()
  for (let step = 0; step < 30; step += 1) {
    if (await separator.evaluate((element) => document.activeElement === element)) break
    await page.keyboard.press('Tab')
  }
  await expect(separator).toBeFocused()
  expect(await separator.evaluate((element) => getComputedStyle(element, '::after').backgroundColor)).not.toBe(
    'rgba(0, 0, 0, 0)',
  )

  await page.reload()
  await expect(page.getByRole('heading', { name: 'Stage inspector' })).toBeVisible()
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(508)
  await expect(separator).toHaveAttribute('aria-valuemax', '508')
  expect(await page.evaluate(() => localStorage.getItem('control-tower-stage-inspector-width-v1'))).toBe('508')
  expect(await page.evaluate(() => Object.keys(localStorage))).toEqual(['control-tower-stage-inspector-width-v1'])
})

test('inspector defaults unusable stored width', async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem('control-tower-stage-inspector-width-v1', 'not-a-width'))
  await installFixtures(page)
  const inspector = page.getByRole('complementary', { name: 'Selected stage inspector' })
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(382)
  expect(await page.evaluate(() => localStorage.getItem('control-tower-stage-inspector-width-v1'))).toBe('not-a-width')
})

test('inspector defaults a zero stored width', async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem('control-tower-stage-inspector-width-v1', '0'))
  await installFixtures(page)
  const inspector = page.getByRole('complementary', { name: 'Selected stage inspector' })
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(382)
  expect(await page.evaluate(() => localStorage.getItem('control-tower-stage-inspector-width-v1'))).toBe('0')
})

test('a valid saved inspector width clamps to narrower geometry without losing the preference', async ({ page }) => {
  const api = await installFixtures(page)
  const inspector = page.getByRole('complementary', { name: 'Selected stage inspector' })
  const separator = page.getByRole('separator', { name: 'Resize stage inspector' })
  const box = await separator.boundingBox()
  if (!box) throw new Error('Resize separator has no rendered bounds')
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2)
  await page.mouse.down()
  await page.mouse.move(box.x - 130, box.y + box.height / 2)
  await page.mouse.up()
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(508)
  expect(await page.evaluate(() => localStorage.getItem('control-tower-stage-inspector-width-v1'))).toBe('508')
  expect(api.movementRequests()).toBe(0)

  await page.setViewportSize({ width: 1200, height: 820 })
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(428)
  await expect(separator).toHaveAttribute('aria-valuenow', '428')
  expect(await page.evaluate(() => localStorage.getItem('control-tower-stage-inspector-width-v1'))).toBe('508')

  await page.setViewportSize({ width: 1280, height: 820 })
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(508)
  await expect(separator).toHaveAttribute('aria-valuenow', '508')

  await page.setViewportSize({ width: 1200, height: 820 })
  await page.reload()
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(428)
  await expect(separator).toHaveAttribute('aria-valuemax', '428')
  expect(await page.evaluate(() => localStorage.getItem('control-tower-stage-inspector-width-v1'))).toBe('508')
})

test('resize bounds follow left-rail geometry and keep the inspector usable at both bounds', async ({ page }) => {
  await installFixtures(page, false, undefined, workflowWithLongContent, workspace, true)
  const inspector = page.getByRole('complementary', { name: 'Selected stage inspector' })
  const separator = page.getByRole('separator', { name: 'Resize stage inspector' })
  await page.setViewportSize({ width: 1000, height: 820 })
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(300)
  await expect(page.getByRole('heading', { name: 'Stage inspector' })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Collapse stage inspector' })).toBeVisible()
  await expect(separator).toHaveAttribute('aria-valuemin', '300')
  await expect(separator).toHaveAttribute('aria-valuemax', '300')
  await separator.press('ArrowRight')
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(300)
  const minimumBox = await separator.boundingBox()
  if (!minimumBox) throw new Error('Resize separator has no rendered bounds')
  await page.mouse.move(minimumBox.x + minimumBox.width / 2, minimumBox.y + minimumBox.height / 2)
  await page.mouse.down()
  await page.mouse.move(minimumBox.x + minimumBox.width / 2 + 80, minimumBox.y + minimumBox.height / 2)
  await page.mouse.up()
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(300)

  await page.setViewportSize({ width: 1400, height: 820 })
  await expect(separator).toHaveAttribute('aria-valuemax', '628')
  const maxBox = await separator.boundingBox()
  if (!maxBox) throw new Error('Resize separator has no rendered bounds')
  await page.mouse.move(maxBox.x + maxBox.width / 2, maxBox.y + maxBox.height / 2)
  await page.mouse.down()
  await page.mouse.move(maxBox.x - 500, maxBox.y + maxBox.height / 2)
  await page.mouse.up()
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(628)
  await expect(separator).toHaveAttribute('aria-valuenow', '628')
  await expect(page.getByRole('button', { name: 'Collapse stage inspector' })).toBeVisible()

  await page.getByRole('button', { name: 'Collapse workflow rail' }).click()
  await expect(separator).toHaveAttribute('aria-valuemax', '820')
  await separator.press('ArrowLeft')
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(644)
  await page.getByRole('button', { name: 'Expand workflow rail' }).click()
  await expect(separator).toHaveAttribute('aria-valuemax', '628')
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(628)
  expect(
    await page
      .locator('[class*="grid"]')
      .evaluate((element) => getComputedStyle(element).gridTemplateColumns.split(' ')[1]),
  ).toBe('520px')
  expect(await page.evaluate(() => localStorage.getItem('control-tower-stage-inspector-width-v1'))).toBe('644')
  await page.getByRole('button', { name: 'Collapse workflow rail' }).click()
  await expect.poll(() => inspector.evaluate((element) => element.getBoundingClientRect().width)).toBe(644)
  await expect(separator).toHaveAttribute('aria-valuenow', '644')
})

test('workflow search filters inventory without changing selection or issuing inventory requests', async ({ page }) => {
  const inventory: WorkspaceView = {
    name: 'browser-fixture',
    workflows: [
      { id: 'fixture', name: 'Alpha Workflow' },
      { id: 'second', name: 'Second long workflow name' },
      { id: 'third', name: 'ALPHABET soup' },
    ],
  }
  const api = await installFixtures(page, false, undefined, { fixture: workflow, second: secondWorkflow }, inventory)
  const startupInventoryRequests = api.inventoryRequests()
  const search = page.getByRole('textbox', { name: 'Search workflows' })
  const workflowRail = page.getByRole('complementary', { name: 'Workspace workflows' })
  await expect(workflowRail.getByText('3', { exact: true })).toBeVisible()
  await search.fill('   ')
  const rowOrder = await Promise.all(
    ['Alpha Workflow', 'Second long workflow name', 'ALPHABET soup'].map(
      async (name) =>
        (await page.getByRole('button', { name, exact: true }).boundingBox())?.y ?? Number.POSITIVE_INFINITY,
    ),
  )
  expect(rowOrder[0]).toBeLessThan(rowOrder[1])
  expect(rowOrder[1]).toBeLessThan(rowOrder[2])
  await expect(workflowRail.getByText('3', { exact: true })).toBeVisible()
  expect(
    await page
      .getByRole('button', { name: 'Alpha Workflow', exact: true })
      .evaluate((el) => getComputedStyle(el).padding),
  ).toBe('0px')
  expect(
    await page.getByRole('button', { name: /Not applied finish/ }).evaluate((el) => getComputedStyle(el).padding),
  ).toBe('0px')
  expect(
    await page
      .getByRole('button', { name: 'Alpha Workflow', exact: true })
      .locator('p')
      .evaluate((el) => getComputedStyle(el).fontSize),
  ).toBe('14px')
  expect(
    await page
      .getByRole('button', { name: /Not applied finish/ })
      .getByText('finish')
      .evaluate((el) => getComputedStyle(el).fontSize),
  ).toBe('14px')
  expect(
    await workflowRail
      .locator(':scope > div')
      .nth(1)
      .evaluate((el) => getComputedStyle(el).paddingLeft),
  ).toBe('12px')
  await search.fill('second')
  await expect(page.getByRole('button', { name: 'Alpha Workflow' })).toHaveCount(0)
  await expect(page.getByRole('heading', { name: 'Alpha Workflow' })).toBeVisible()
  await expect(page.getByText('Stage 10 · seed')).toBeVisible()
  await expect(page.getByRole('button', { name: 'Second long workflow name' })).toBeVisible()
  await page.getByRole('button', { name: 'Second long workflow name' }).click()
  await expect(page.getByRole('heading', { name: 'Second long workflow name' })).toBeVisible()
  await expect(page.getByText('Stage 10 · beta checkpoint')).toBeVisible()
  await expect(page.getByRole('heading', { name: 'beta checkpoint' })).toBeVisible()
  const selectedWorkflowCounts = {
    inventory: api.inventoryRequests(),
    workflowReads: api.workflowReadRequests(),
    definitions: api.definitionRequests(),
    eventSources: await api.eventSourceRequests(),
  }

  await search.fill('alpha')
  await expect(page.getByRole('button', { name: 'Second long workflow name' })).toHaveCount(0)
  await expect(page.getByRole('heading', { name: 'Second long workflow name' })).toBeVisible()
  await expect(page.getByText('Stage 10 · beta checkpoint')).toBeVisible()
  await expect(page.getByRole('heading', { name: 'beta checkpoint' })).toBeVisible()
  await expect(workflowRail.getByText('3', { exact: true })).toBeVisible()
  await page.getByRole('button', { name: 'Collapse workflow rail' }).click()
  await expect(page.getByRole('button', { name: 'Select workflow Alpha Workflow' })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Select workflow Second long workflow name' })).toHaveCount(0)
  await page.getByRole('button', { name: 'Expand workflow rail' }).click()
  await expect(search).toHaveValue('alpha')
  await search.fill('nothing matches')
  await expect(page.getByText('No matching workflows')).toBeVisible()
  await search.clear()
  await expect(page.getByRole('button', { name: 'Second long workflow name' })).toHaveAttribute('aria-current', 'page')
  expect(api.inventoryRequests()).toBe(selectedWorkflowCounts.inventory)
  expect(api.workflowReadRequests()).toBe(selectedWorkflowCounts.workflowReads)
  expect(api.definitionRequests()).toBe(selectedWorkflowCounts.definitions)
  expect(await api.eventSourceRequests()).toEqual(selectedWorkflowCounts.eventSources)
  expect(api.movementRequests()).toBe(0)
  expect(api.inventoryRequests()).toBe(startupInventoryRequests)

  await search.fill('aLpHa')
  await expect(page.getByRole('button', { name: 'ALPHABET soup' })).toBeVisible()
  await page.getByRole('button', { name: 'Alpha Workflow' }).click()
  await expect(page.getByRole('heading', { name: 'Alpha Workflow' })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Alpha Workflow' })).toHaveAttribute('aria-current', 'page')
  expect(api.movementRequests()).toBe(0)
  await page.reload()
  await expect(page.getByRole('textbox', { name: 'Search workflows' })).toHaveValue('')
  await expect(page.getByRole('button', { name: 'Alpha Workflow' })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Second long workflow name' })).toBeVisible()
  await expect(page.getByRole('button', { name: 'ALPHABET soup' })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Alpha Workflow' })).toHaveAttribute('aria-current', 'page')
  await expect(workflowRail.getByText('3', { exact: true })).toBeVisible()
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
