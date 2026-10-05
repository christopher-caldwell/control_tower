import { skipToken, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useEffect, useRef, useState } from 'react'

import { ApiFailure } from '@/api/client'
import type { WorkflowView } from '@/api/types'
import { definitionOptions, keys, movementOptions, workspaceOptions } from '@/features/workflows/workbench/api/options'
import { parseSnapshot } from '@/features/workflows/workbench/api/snapshot'
import type { Action } from '@/features/workflows/workbench/presentation'
import { forwardActions, movementActions } from '@/features/workflows/workbench/presentation'

const autoStage = (view: WorkflowView) =>
  [
    view.observation?.active_role?.stage.number,
    view.observation?.failure?.stage?.number,
    view.checkpoint?.pending_transition?.stage.number,
    view.checkpoint?.accepted_stage?.number,
    view.stages[0]?.number,
  ].find((number) => number !== undefined && view.stages.some((stage) => stage.number === number)) ?? null
const message = (error: unknown) => (error instanceof Error ? error.message : 'Request failed')

export const useWorkbench = () => {
  const client = useQueryClient()
  const workspaceQuery = useQuery(workspaceOptions)
  const [selection, setSelection] = useState<string | null>(null)
  const workflowId =
    selection && workspaceQuery.data?.workflows.some((item) => item.id === selection)
      ? selection
      : (workspaceQuery.data?.workflows[0]?.id ?? null)
  const currentId = useRef(workflowId)
  const [inspection, setInspection] = useState<{ id: string; number: number } | null>(null)
  const [connection, setConnection] = useState<{ id: string | null; ready: boolean; label: string }>({
    id: null,
    ready: false,
    label: 'Connecting',
  })
  const [revision, setRevision] = useState(0)
  const [pending, setPending] = useState<{ id: string; label: string } | null>(null)
  const [issue, setIssue] = useState<{ id: string; message: string } | null>(null)
  const request = useRef(0)
  const admission = useRef(false)
  const phase = useRef<'idle' | 'posting' | 'reconciling'>('idle')
  const mutation = useMutation(movementOptions)
  const snapshot = useQuery<WorkflowView>({ queryKey: keys.workflow(workflowId), queryFn: skipToken })
  const workflow = snapshot.data
  const manual = inspection?.id === workflowId && workflow?.stages.some((stage) => stage.number === inspection.number)
  const stageNumber = manual ? inspection.number : workflow ? autoStage(workflow) : null
  const definitionQuery = useQuery(definitionOptions(workflowId, stageNumber))

  useEffect(() => {
    currentId.current = workflowId
    if (!workflowId) return
    let closed = false
    const source = new EventSource(`/api/workflows/${encodeURIComponent(workflowId)}/events`)
    setConnection({ id: workflowId, ready: false, label: 'Connecting' })
    source.onopen = () => {
      if (!closed) setConnection({ id: workflowId, ready: false, label: 'Synchronizing' })
    }
    source.onerror = () => {
      if (!closed) {
        setConnection({ id: workflowId, ready: false, label: 'Reconnecting' })
        if (phase.current !== 'posting') {
          setPending(null)
          admission.current = false
          phase.current = 'idle'
        }
      }
    }
    source.addEventListener('snapshot', (event) => {
      if (closed || currentId.current !== workflowId) return
      try {
        const next = parseSnapshot((event as MessageEvent<string>).data, workflowId)
        client.setQueryData(keys.workflow(workflowId), next)
        setInspection((previous) =>
          previous?.id === workflowId && !next.stages.some((stage) => stage.number === previous.number)
            ? null
            : previous,
        )
        setConnection({ id: workflowId, ready: true, label: 'Live updates connected' })
        if (phase.current !== 'posting') {
          setPending((previous) => (previous?.id === workflowId ? null : previous))
          admission.current = false
          phase.current = 'idle'
        }
      } catch {
        setConnection({ id: workflowId, ready: false, label: 'Resynchronizing' })
      }
    })
    return () => {
      closed = true
      source.close()
      if (currentId.current === workflowId) currentId.current = null
    }
  }, [client, workflowId, revision])

  const ready = connection.id === workflowId && connection.ready
  const submitting = pending?.id === workflowId ? pending : null
  const busy = !ready || Boolean(submitting) || Boolean(workflow?.movement_busy)
  const actions = workflow?.current_status === 'available' ? movementActions(workflow) : []
  const forward =
    workflow?.current_status === 'available'
      ? forwardActions(workflow, stageNumber)
      : { next: null, to: null, all: null }
  const refresh = () => {
    setConnection({ id: workflowId, ready: false, label: 'Synchronizing' })
    setRevision((value) => value + 1)
  }
  const run = async (action: Action) => {
    if (!workflowId || !workflow?.checkpoint || busy || admission.current || workflow.current_status !== 'available')
      return
    const id = workflowId
    const token = ++request.current
    admission.current = true
    phase.current = 'posting'
    setPending({ id, label: action.label })
    setIssue(null)
    try {
      await mutation.mutateAsync({ id, choice: action.choice, checkpoint: workflow.checkpoint.state })
    } catch (error) {
      if (currentId.current === id && request.current === token) {
        setIssue({
          id,
          message:
            error instanceof ApiFailure && error.code === 'stale_checkpoint'
              ? 'Checkpoint changed before the movement. Refreshing the live view; review it before submitting again.'
              : `Movement response was not confirmed: ${message(error)}. Check live status before submitting again.`,
        })
        setPending(null)
      }
    } finally {
      if (currentId.current === id && request.current === token) {
        phase.current = 'reconciling'
        refresh()
      }
    }
  }
  const selectWorkflow = (id: string) => {
    if (id === workflowId) return
    currentId.current = id
    request.current += 1
    admission.current = false
    phase.current = 'idle'
    setInspection(null)
    setPending(null)
    setIssue(null)
    setConnection({ id, ready: false, label: 'Connecting' })
    setSelection(id)
  }
  return {
    workspace: workspaceQuery.data,
    workspaceLoading: workspaceQuery.isPending,
    workspaceIssue: workspaceQuery.error ? message(workspaceQuery.error) : null,
    retryWorkspace: () => void workspaceQuery.refetch(),
    workflowId,
    workflow,
    selectedSummary: workflow?.workflow ?? workspaceQuery.data?.workflows.find((item) => item.id === workflowId),
    stageNumber,
    selectedStage: workflow?.stages.find((stage) => stage.number === stageNumber) ?? null,
    definition: definitionQuery.data,
    definitionLoading: stageNumber !== null && definitionQuery.isPending,
    definitionIssue: definitionQuery.error ? message(definitionQuery.error) : null,
    retryDefinition: () => void definitionQuery.refetch(),
    live: workflowId ? (connection.id === workflowId ? connection.label : 'Connecting') : 'Disconnected',
    ready,
    busy,
    pending: submitting,
    movementIssue: issue?.id === workflowId ? issue.message : null,
    actions,
    forward,
    run,
    refresh,
    selectWorkflow,
    chooseStage: (number: number) => {
      if (workflowId) setInspection({ id: workflowId, number })
    },
  }
}
