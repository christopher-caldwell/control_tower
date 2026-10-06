import { act, cleanup, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { MovementObservation, RoleObservation, WorkflowView, WorkspaceView } from '@/api/types'
import { App } from '@/app/app'

const stage = { number: 10, name: 'seed' }
const laterStage = { number: 20, name: 'finish' }

class ControlledEventSource extends EventTarget {
  static instances: ControlledEventSource[] = []
  constructor(
    readonly url: string,
    _init?: EventSourceInit,
  ) {
    super()
    ControlledEventSource.instances.push(this)
  }
  onopen: (() => void) | null = null
  onerror: (() => void) | null = null
  closed = false
  close() {
    this.closed = true
  }
  snapshot(value: WorkflowView) {
    this.onopen?.()
    this.dispatchEvent(new MessageEvent('snapshot', { data: JSON.stringify(value) }))
  }
}

function result(overrides: Partial<RoleObservation> = {}): RoleObservation {
  return {
    stage,
    role: 'up',
    state: 'succeeded',
    exit_code: 0,
    message: null,
    stdout: '',
    stderr: '',
    ...overrides,
  }
}

function observation(
  roleResults: RoleObservation[] = [result()],
  state: MovementObservation['state'] = 'complete',
): MovementObservation {
  return {
    direction: 'up',
    target_stage: 10,
    state,
    active_role: null,
    role_results: roleResults,
    failure: null,
    verification_choices: null,
  }
}

function workflow(id: string, overrides: Partial<WorkflowView> = {}): WorkflowView {
  return {
    workspace_name: 'preview-fixture',
    workflow: { id, name: id },
    current_status: 'available',
    status_issue: null,
    checkpoint: {
      accepted_stage: stage,
      pending_transition: null,
      state: { completed_stage_count: 1, uuid: 'fixture-run', pending: null },
    },
    movement_choices: [
      { direction: 'up', target_stage: 20 },
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
        number: 20,
        name: 'finish',
        state: 'future',
        is_accepted_checkpoint: false,
        definitions: [{ role: 'up', path: 'stages/020-finish/up' }],
      },
    ],
    movement_busy: false,
    observation: observation(),
    ...overrides,
  }
}

function workspaceFor(ids: string[]): WorkspaceView {
  return { name: 'preview-fixture', workflows: ids.map((id) => ({ id, name: id })) }
}

function installWorkbench(
  snapshots: Record<string, WorkflowView>,
  handler?: (url: string, init?: RequestInit) => Promise<Response> | undefined,
) {
  ControlledEventSource.instances = []
  const fetchMock = vi.fn((input: RequestInfo | URL, _init?: RequestInit) => {
    const url = String(input)
    const handled = handler?.(url, _init)
    if (handled) return handled
    if (url === '/api/workspace') return Promise.resolve(Response.json(workspaceFor(Object.keys(snapshots))))
    const stageNumber = url.match(/\/stages\/(\d+)$/)?.[1]
    if (stageNumber)
      return Promise.resolve(Response.json({ stage: Number(stageNumber) === 20 ? laterStage : stage, definitions: [] }))
    return Promise.resolve(Response.json({}, { status: 404 }))
  })
  vi.stubGlobal('fetch', fetchMock)
  vi.stubGlobal('EventSource', ControlledEventSource)
  render(<App />)
  return fetchMock
}

async function emitSnapshot(id: string, snapshot: WorkflowView) {
  await waitFor(() =>
    expect(
      ControlledEventSource.instances.some((source) => source.url.includes('/api/workflows/' + id + '/events')),
    ).toBe(true),
  )
  const source = [...ControlledEventSource.instances]
    .reverse()
    .find((item) => !item.closed && item.url.includes('/api/workflows/' + id + '/events'))
  if (!source) throw new Error('No event source for ' + id)
  act(() => source.snapshot(snapshot))
  await screen.findByRole('heading', { name: 'Stage inspector' })
}

afterEach(() => {
  cleanup()
  vi.unstubAllGlobals()
  ControlledEventSource.instances = []
})

describe('workflow browser adapter', () => {
  it('filters local workflow names without changing a hidden selection or refreshing inventory', async () => {
    const first = workflow('alpha')
    const second = workflow('beta', {
      checkpoint: {
        accepted_stage: { number: 10, name: 'beta checkpoint' },
        pending_transition: null,
        state: { completed_stage_count: 1, uuid: 'beta-run', pending: null },
      },
      stages: [
        {
          number: 10,
          name: 'beta checkpoint',
          state: 'accepted',
          is_accepted_checkpoint: true,
          definitions: [{ role: 'up', path: 'stages/010-beta/up' }],
        },
        {
          number: 20,
          name: 'beta finish',
          state: 'future',
          is_accepted_checkpoint: false,
          definitions: [{ role: 'up', path: 'stages/020-beta-finish/up' }],
        },
      ],
    })
    const fetchMock = installWorkbench({ alpha: first, beta: second })
    await screen.findByRole('button', { name: 'beta' })
    await userEvent.setup().click(screen.getByRole('button', { name: 'beta' }))
    await emitSnapshot('beta', second)
    expect(screen.getByRole('heading', { name: 'beta' })).toBeInTheDocument()
    expect(screen.getByText('Stage 10 · beta checkpoint')).toBeInTheDocument()
    expect(screen.getByRole('heading', { name: 'beta checkpoint' })).toBeInTheDocument()

    const search = screen.getByRole('textbox', { name: 'Search workflows' })
    await userEvent.setup().type(search, 'no match')
    expect(screen.getByText('No matching workflows')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'beta' })).not.toBeInTheDocument()
    expect(screen.getByRole('heading', { name: 'beta' })).toBeInTheDocument()
    expect(screen.getByText('Stage 10 · beta checkpoint')).toBeInTheDocument()
    expect(screen.getByRole('heading', { name: 'beta checkpoint' })).toBeInTheDocument()
    const selectedEventSourceCount = ControlledEventSource.instances.length
    const selectedDefinitionRequestCount = fetchMock.mock.calls.filter(([url]) =>
      String(url).includes('/stages/'),
    ).length
    expect(fetchMock.mock.calls.filter(([url]) => String(url) === '/api/workspace')).toHaveLength(1)
    expect(fetchMock.mock.calls.some(([url]) => String(url).includes('/movements'))).toBe(false)

    await userEvent.setup().clear(search)
    expect(await screen.findByRole('button', { name: 'beta' })).toHaveAttribute('aria-current', 'page')
    expect(ControlledEventSource.instances).toHaveLength(selectedEventSourceCount)
    expect(fetchMock.mock.calls.filter(([url]) => String(url).includes('/stages/'))).toHaveLength(
      selectedDefinitionRequestCount,
    )
    await userEvent.setup().click(screen.getByRole('button', { name: 'alpha' }))
    await emitSnapshot('alpha', first)
    expect(screen.getByRole('heading', { name: 'alpha' })).toBeInTheDocument()
    expect(fetchMock.mock.calls.some(([, init]) => init?.method === 'POST')).toBe(false)
  })

  it('waits for established inventory before showing an empty workspace', async () => {
    const response = deferred<Response>()
    installWorkbench({}, (url) => (url === '/api/workspace' ? response.promise : undefined))

    expect(screen.getByText('Discovering workflows…')).toBeInTheDocument()
    expect(screen.queryByText(/No workflow directories were found/)).not.toBeInTheDocument()
    expect(screen.queryByRole('heading', { name: 'No workflows found' })).not.toBeInTheDocument()

    await act(async () => response.resolve(Response.json(workspaceFor([]))))
    expect(await screen.findByRole('heading', { name: 'No workflows found' })).toBeInTheDocument()
    expect(screen.getByText(/No workflow directories were found/)).toBeInTheDocument()
    expect(screen.queryByText('Discovering workflows…')).not.toBeInTheDocument()
  })

  it.each([{ ids: [] }, { ids: ['healthy'] }])(
    'reports initial inventory failure and retries to $ids',
    async ({ ids }) => {
      let inventoryRequests = 0
      installWorkbench({}, (url) => {
        if (url !== '/api/workspace') return undefined
        inventoryRequests += 1
        const response =
          inventoryRequests === 1
            ? Response.json({ error: { code: 'workspace_failed', message: 'Cannot read workspace' } }, { status: 503 })
            : Response.json(workspaceFor(ids))
        return Promise.resolve(response)
      })

      expect(await screen.findByRole('alert')).toHaveTextContent('Cannot read workspace')
      expect(screen.queryByText(/No workflow directories were found/)).not.toBeInTheDocument()
      expect(screen.queryByRole('heading', { name: 'No workflows found' })).not.toBeInTheDocument()
      expect(inventoryRequests).toBe(1)

      await userEvent.setup().click(screen.getByRole('button', { name: 'Retry workspace' }))
      if (ids.length === 0) {
        expect(await screen.findByRole('heading', { name: 'No workflows found' })).toBeInTheDocument()
        expect(screen.getByText(/No workflow directories were found/)).toBeInTheDocument()
      } else {
        expect(await screen.findByRole('button', { name: 'healthy' })).toBeInTheDocument()
        expect(screen.queryByText(/No workflow directories were found/)).not.toBeInTheDocument()
        expect(screen.queryByRole('heading', { name: 'No workflows found' })).not.toBeInTheDocument()
      }
      expect(screen.queryByRole('alert')).not.toBeInTheDocument()
      expect(inventoryRequests).toBe(2)
    },
  )

  it('lists directory identities without claiming every workflow is ready', async () => {
    const healthy = workflow('healthy')
    const unprepared = workflow('unprepared', {
      current_status: 'unavailable',
      status_issue: 'no such table: checkpoint',
      checkpoint: null,
      movement_choices: [],
      stages: [],
      observation: null,
    })
    installWorkbench({ healthy, unprepared })
    expect(await screen.findByRole('button', { name: /healthy/ })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /unprepared/ })).toBeInTheDocument()

    await userEvent.setup().click(screen.getByRole('button', { name: /unprepared/ }))
    await emitSnapshot('unprepared', unprepared)
    expect(screen.getByRole('alert')).toHaveTextContent('no such table: checkpoint')
  })

  it('keeps the last observed role output readable while current status is unavailable', async () => {
    const snapshot = workflow('workflow-a', {
      current_status: 'unavailable',
      status_issue: 'unable to open database file',
      checkpoint: null,
      movement_choices: [],
      stages: [],
      observation: observation([
        result({ stdout: 'mutation out', stderr: 'mutation err' }),
        result({ role: 'verify-up', stdout: 'verifier out', stderr: 'verifier err' }),
      ]),
    })
    installWorkbench({ 'workflow-a': snapshot })
    await emitSnapshot('workflow-a', snapshot)

    expect(screen.getByRole('alert')).toHaveTextContent('unable to open database file')
    expect(screen.getAllByLabelText('stdout').map((element) => element.textContent)).toEqual([
      'mutation out',
      'verifier out',
    ])
    expect(screen.getAllByLabelText('stderr').map((element) => element.textContent)).toEqual([
      'mutation err',
      'verifier err',
    ])
    expect(screen.queryByText('Select a stage')).not.toBeInTheDocument()
    expect(screen.queryByText('CURRENT CHECKPOINT')).not.toBeInTheDocument()
    expect(screen.getByText('Workflow unavailable')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Workflow unavailable' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /Advance|Back out/ })).not.toBeInTheDocument()
  })

  it('renders buffered stdout and stderr as inert text from the selected workflow snapshot', async () => {
    const snapshot = workflow('workflow-a', {
      observation: observation([
        result({ stdout: '<script>window.fixture=true</script>', stderr: 'separate error text' }),
      ]),
    })
    const fetchMock = installWorkbench({ 'workflow-a': snapshot })
    await emitSnapshot('workflow-a', snapshot)

    const stdout = screen.getByLabelText('stdout')
    const stderr = screen.getByLabelText('stderr')
    expect(stdout).toHaveTextContent('<script>window.fixture=true</script>')
    expect(stdout.querySelector('script')).toBeNull()
    expect(stderr).toHaveTextContent('separate error text')
    expect(fetchMock.mock.calls.some(([url]) => String(url).includes('/outputs/'))).toBe(false)
  })

  it('keeps an accepted checkpoint separate from unrecorded role results', async () => {
    const snapshot = workflow('workflow-a', {
      stages: [
        {
          number: 10,
          name: 'seed',
          state: 'accepted',
          is_accepted_checkpoint: true,
          definitions: [
            { role: 'up', path: 'stages/010-seed/up' },
            { role: 'down', path: 'stages/010-seed/down' },
            { role: 'verify-up', path: 'stages/010-seed/verify-up' },
            { role: 'verify-down', path: 'stages/010-seed/verify-down' },
          ],
        },
      ],
      observation: observation([result({ role: 'verify-up' })]),
    })
    installWorkbench({ 'workflow-a': snapshot })
    await emitSnapshot('workflow-a', snapshot)

    expect(screen.getAllByText('Applied').length).toBeGreaterThan(0)
    const roleStatus = (role: string) => {
      return screen.getByLabelText(`${role} result`).textContent?.replace(role, '')
    }
    expect(roleStatus('up')).toBe('Applied · history unavailable')
    expect(roleStatus('verify-up')).toBe('Process OK')
    expect(roleStatus('down')).toBe('No recorded result')
    expect(roleStatus('verify-down')).toBe('No recorded result')
  })

  it('lets stage selection inspect a future definition without submitting movement', async () => {
    const user = userEvent.setup()
    const snapshot = workflow('workflow-a')
    const fetchMock = installWorkbench({ 'workflow-a': snapshot })
    await emitSnapshot('workflow-a', snapshot)

    await user.click(screen.getByRole('button', { name: /Not applied finish/i }))
    expect(await screen.findByRole('heading', { name: 'finish' })).toBeInTheDocument()
    expect(fetchMock.mock.calls.some(([url]) => String(url).includes('/stages/20'))).toBe(true)
    expect(fetchMock.mock.calls.some(([, init]) => (init as RequestInit | undefined)?.method === 'POST')).toBe(false)
  })

  it('distinguishes output still buffered during a role from an executable launch error', async () => {
    const snapshot = workflow('workflow-a', {
      observation: observation(
        [result({ state: 'in_progress', exit_code: null, stdout: null, stderr: null })],
        'running',
      ),
    })
    installWorkbench({ 'workflow-a': snapshot })
    await emitSnapshot('workflow-a', snapshot)
    expect(screen.getByText('Output will be available when the role returns.')).toBeInTheDocument()

    const failed = workflow('workflow-a', {
      observation: observation(
        [
          result({
            state: 'launch_failed',
            exit_code: null,
            message: 'permission denied by fixture',
            stdout: null,
            stderr: null,
          }),
        ],
        'stopped',
      ),
    })
    await emitSnapshot('workflow-a', failed)
    expect(screen.getAllByText('permission denied by fixture').length).toBeGreaterThan(0)
    expect(screen.queryByText('Output will be available when the role returns.')).not.toBeInTheDocument()
  })
})

const latestSource = () => {
  const source = [...ControlledEventSource.instances].reverse().find((item) => !item.closed)
  if (!source) throw new Error('No active subscription')
  return source
}
const deferred = <T,>() => {
  let resolve!: (value: T) => void
  let reject!: (reason: unknown) => void
  const promise = new Promise<T>((res, rej) => {
    resolve = res
    reject = rej
  })
  return { promise, resolve, reject }
}

describe('live workbench reconciliation', () => {
  it('preserves manual inspection while the checkpoint advances', async () => {
    const initial = workflow('a')
    installWorkbench({ a: initial })
    await emitSnapshot('a', initial)
    await userEvent.setup().click(screen.getByRole('button', { name: /Not applied finish/ }))
    await emitSnapshot('a', workflow('a', { observation: null }))
    expect(screen.getByRole('heading', { name: 'finish' })).toBeInTheDocument()
  })

  it('requires a fresh snapshot after disconnect and ignores malformed events', async () => {
    const initial = workflow('a')
    installWorkbench({ a: initial })
    await emitSnapshot('a', initial)
    const next = screen.getByRole('button', { name: 'Run next' })
    expect(next).toBeEnabled()
    act(() => latestSource().onerror?.())
    expect(next).toBeDisabled()
    expect(screen.getByRole('heading', { name: 'seed' })).toBeInTheDocument()
    expect(screen.queryByText('Movement is in progress')).not.toBeInTheDocument()
    act(() => latestSource().onopen?.())
    expect(next).toBeDisabled()
    act(() => latestSource().dispatchEvent(new MessageEvent('snapshot', { data: '{invalid' })))
    expect(next).toBeDisabled()
    act(() =>
      latestSource().dispatchEvent(
        new MessageEvent('snapshot', {
          data: JSON.stringify({ ...initial, observation: { ...initial.observation, role_results: null } }),
        }),
      ),
    )
    expect(next).toBeDisabled()
    expect(screen.getByRole('heading', { name: 'seed' })).toBeInTheDocument()
    await emitSnapshot('a', initial)
    expect(next).toBeEnabled()
  })

  it('emphasizes Run next and renders unavailable movement as noninteractive status', async () => {
    const initial = workflow('a')
    installWorkbench({ a: initial })
    await emitSnapshot('a', initial)
    expect(screen.getByRole('button', { name: 'Run next' })).toHaveAttribute('data-variant', 'filled')

    const unavailable = workflow('a', { movement_choices: [] })
    await emitSnapshot('a', unavailable)
    expect(screen.getByText('No immediate movement available')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'No immediate movement available' })).not.toBeInTheDocument()
  })

  it('keeps the initiating action pending through POST and the renewed snapshot', async () => {
    const response = deferred<Response>()
    const initial = workflow('a')
    const fetchMock = installWorkbench({ a: initial }, (url) =>
      url.endsWith('/movements') ? response.promise : undefined,
    )
    await emitSnapshot('a', initial)
    const next = screen.getByRole('button', { name: 'Run next' })
    await userEvent.setup().click(next)
    expect(next).toHaveAttribute('data-loading', 'true')
    await emitSnapshot('a', initial)
    expect(next).toHaveAttribute('data-loading', 'true')
    const old = latestSource()
    await act(async () => response.resolve(new Response(null, { status: 204 })))
    await waitFor(() => expect(old.closed).toBe(true))
    expect(next).toHaveAttribute('data-loading', 'true')
    expect(next).toBeDisabled()
    act(() =>
      latestSource().dispatchEvent(
        new MessageEvent('snapshot', {
          data: JSON.stringify({ ...initial, observation: { ...initial.observation, role_results: null } }),
        }),
      ),
    )
    expect(next).toBeDisabled()
    expect(screen.getByRole('heading', { name: 'seed' })).toBeInTheDocument()
    await emitSnapshot('a', initial)
    expect(next).not.toHaveAttribute('data-loading')
    expect(next).toBeEnabled()
    expect(fetchMock.mock.calls.filter(([, init]) => init?.method === 'POST')).toHaveLength(1)
  })

  it('does not leak a late movement error or obsolete snapshots into another workflow', async () => {
    const response = deferred<Response>()
    const a = workflow('a'),
      b = workflow('b', { observation: null })
    installWorkbench({ a, b }, (url) => (url.endsWith('/movements') ? response.promise : undefined))
    await emitSnapshot('a', a)
    await userEvent.setup().click(screen.getByRole('button', { name: 'Run next' }))
    const old = latestSource()
    await userEvent.setup().click(screen.getByRole('button', { name: 'b' }))
    await emitSnapshot('b', b)
    expect(old.closed).toBe(true)
    act(() => old.snapshot(workflow('a', { status_issue: 'obsolete error' })))
    await act(async () => response.reject(new Error('late delivery failure')))
    expect(screen.queryByText(/late delivery failure|obsolete error/)).not.toBeInTheDocument()
    expect(screen.getByRole('heading', { name: 'b' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Run next' })).toBeEnabled()
  })

  it('aborts obsolete definition reads and never shows the old workflow definition', async () => {
    const response = deferred<Response>()
    let signal: AbortSignal | undefined | null
    const a = workflow('a'),
      b = workflow('b')
    installWorkbench({ a, b }, (url, init) => {
      if (url.includes('/a/stages/')) {
        signal = init?.signal
        return response.promise
      }
      return undefined
    })
    await emitSnapshot('a', a)
    await waitFor(() => expect(signal).toBeDefined())
    await userEvent.setup().click(screen.getByRole('button', { name: 'b' }))
    await emitSnapshot('b', b)
    expect(signal?.aborted).toBe(true)
    await act(async () =>
      response.resolve(
        Response.json({
          stage,
          definitions: [{ role: 'up', path: 'old', contents: 'OBSOLETE DEFINITION', issue: null }],
        }),
      ),
    )
    expect(screen.queryByText('OBSOLETE DEFINITION')).not.toBeInTheDocument()
  })

  it('retains stale-checkpoint feedback after fresh snapshots without resubmitting', async () => {
    const initial = workflow('a')
    const fetchMock = installWorkbench({ a: initial }, (url) =>
      url.endsWith('/movements')
        ? Promise.resolve(Response.json({ error: { code: 'stale_checkpoint', message: 'changed' } }, { status: 409 }))
        : undefined,
    )
    await emitSnapshot('a', initial)
    await userEvent.setup().click(screen.getByRole('button', { name: 'Run next' }))
    await screen.findByText(/Checkpoint changed before the movement/)
    await emitSnapshot('a', initial)
    expect(screen.getByText(/Checkpoint changed before the movement/)).toBeInTheDocument()
    expect(fetchMock.mock.calls.filter(([, init]) => init?.method === 'POST')).toHaveLength(1)
  })

  it('uses only Application recovery choices when verification failed', async () => {
    const pending = workflow('a', {
      checkpoint: {
        accepted_stage: stage,
        pending_transition: { direction: 'up', stage: laterStage },
        state: { completed_stage_count: 1, uuid: 'fixture-run', pending: { stage_index: 1, direction: 'up' } },
      },
      stages: workflow('a').stages.map((item) => (item.number === 20 ? { ...item, state: 'pending' } : item)),
      observation: {
        ...observation([], 'stopped'),
        failure: { kind: 'verification_failed', stage: laterStage, role: 'verify-up', message: 'Verifier rejected' },
        verification_choices: {
          retry: { direction: 'up', target_stage: 20 },
          reverse: { direction: 'down', target_stage: 10 },
        },
      },
    })
    let submitted: unknown
    installWorkbench({ a: pending }, (url, init) => {
      if (!url.endsWith('/movements')) return undefined
      submitted = JSON.parse(init!.body as string)
      return Promise.resolve(new Response(null, { status: 204 }))
    })
    await emitSnapshot('a', pending)
    expect(screen.queryByRole('button', { name: 'Run next' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Run all' })).not.toBeInTheDocument()
    await userEvent.setup().click(screen.getByRole('button', { name: 'Retry verify-up for Stage 20' }))
    expect(submitted).toEqual({ direction: 'up', target_stage: 20, expected_checkpoint: pending.checkpoint!.state })
  })

  it('keeps the inspector collapsed when another stage is selected', async () => {
    const initial = workflow('a')
    installWorkbench({ a: initial })
    await emitSnapshot('a', initial)
    await userEvent.setup().click(screen.getByRole('button', { name: 'Collapse stage inspector' }))
    await userEvent.setup().click(screen.getByRole('button', { name: /Not applied finish/ }))
    expect(screen.queryByRole('heading', { name: 'Stage inspector' })).not.toBeInTheDocument()
    await userEvent.setup().click(screen.getByRole('button', { name: 'Expand stage inspector' }))
    expect(screen.getByRole('heading', { name: 'finish' })).toBeInTheDocument()
  })
})
