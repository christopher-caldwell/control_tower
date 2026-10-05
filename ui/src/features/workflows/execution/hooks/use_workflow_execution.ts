import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useEffect, useLayoutEffect, useRef, useState } from 'react'

import { ApiFailure } from '@/api/client'
import type { WorkflowView } from '@/api/types'
import {
  submitWorkflowMovement,
  workflowKey,
  workflowSnapshotOptions,
} from '@/features/workflows/execution/api/options'
import { parseSnapshot } from '@/features/workflows/execution/api/snapshot'
import type { Action } from '@/features/workflows/execution/model/movement_actions'
import { forwardActions, movementActions } from '@/features/workflows/execution/model/movement_actions'

export type UseWorkflowExecutionOptions = { workflowId: string | null }
export type WorkflowExecutionModel = ReturnType<typeof useWorkflowExecution>

type Connection = { workflowId: string | null; isReady: boolean; label: string }
type PendingMovement = { workflowId: string; label: string }
type MovementIssue = { workflowId: string; message: string }

const messageFrom = (error: unknown): string => (error instanceof Error ? error.message : 'Request failed')
const connectionLabel = (workflowId: string | null, connection: Connection): string => {
  if (!workflowId) return 'Disconnected'
  return connection.workflowId === workflowId ? connection.label : 'Connecting'
}

const automaticStageNumber = (workflow: WorkflowView): number | null => {
  const candidates = [
    workflow.observation?.active_role?.stage.number,
    workflow.observation?.failure?.stage?.number,
    workflow.checkpoint?.pending_transition?.stage.number,
    workflow.checkpoint?.accepted_stage?.number,
    workflow.stages[0]?.number,
  ]
  return (
    candidates.find((number) => number !== undefined && workflow.stages.some((stage) => stage.number === number)) ??
    null
  )
}

const inspectionIsValid = (
  workflow: WorkflowView | undefined,
  workflowId: string | null,
  inspection: InspectionSelection | null,
) => {
  if (!workflow || !workflowId || !inspection) return false
  if (inspection.workflowId !== workflowId) return false
  return workflow.stages.some((stage) => stage.number === inspection.stageNumber)
}

type InspectionSelection = { workflowId: string; stageNumber: number }

export const useWorkflowExecution = ({ workflowId }: UseWorkflowExecutionOptions) => {
  const queryClient = useQueryClient()
  const workflowQuery = useQuery(workflowSnapshotOptions(workflowId))
  const workflow = workflowQuery.data
  const [inspection, setInspection] = useState<InspectionSelection | null>(null)
  const [connection, setConnection] = useState<Connection>({ workflowId: null, isReady: false, label: 'Connecting' })
  const [revision, setRevision] = useState(0)
  const [pending, setPending] = useState<PendingMovement | null>(null)
  const [issue, setIssue] = useState<MovementIssue | null>(null)
  const currentWorkflowId = useRef(workflowId)
  const requestNumber = useRef(0)
  const movementAdmitted = useRef(false)
  const movementPhase = useRef<'idle' | 'posting' | 'reconciling'>('idle')
  const movementMutation = useMutation({ mutationFn: submitWorkflowMovement, retry: false })

  const previousWorkflowId = useRef(workflowId)
  useLayoutEffect(() => {
    if (previousWorkflowId.current !== workflowId) {
      previousWorkflowId.current = workflowId
      currentWorkflowId.current = workflowId
      requestNumber.current += 1
      movementAdmitted.current = false
      movementPhase.current = 'idle'
      setPending(null)
      setIssue(null)
      setInspection(null)
      return
    }
    currentWorkflowId.current = workflowId
  }, [workflowId])

  const hasManualInspection = inspectionIsValid(workflow, workflowId, inspection)
  let selectedStageNumber: number | null = null
  if (hasManualInspection && inspection) selectedStageNumber = inspection.stageNumber
  else if (workflow) selectedStageNumber = automaticStageNumber(workflow)
  const selectedStage = workflow?.stages.find((stage) => stage.number === selectedStageNumber) ?? null

  useEffect(() => {
    if (!workflowId) {
      setConnection({ workflowId: null, isReady: false, label: 'Disconnected' })
      return
    }
    let isClosed = false
    const source = new EventSource('/api/workflows/' + encodeURIComponent(workflowId) + '/events')
    setConnection({ workflowId, isReady: false, label: 'Connecting' })
    source.onopen = () => {
      if (!isClosed) setConnection({ workflowId, isReady: false, label: 'Synchronizing' })
    }
    source.onerror = () => {
      if (isClosed) return
      setConnection({ workflowId, isReady: false, label: 'Reconnecting' })
      if (movementPhase.current === 'posting') return
      setPending(null)
      movementAdmitted.current = false
      movementPhase.current = 'idle'
    }
    source.addEventListener('snapshot', (event) => {
      if (isClosed || currentWorkflowId.current !== workflowId) return
      try {
        const next = parseSnapshot((event as MessageEvent<string>).data, workflowId)
        queryClient.setQueryData(workflowKey(workflowId), next)
        setInspection((previous) => {
          const isSelectedWorkflow = previous?.workflowId === workflowId
          const inspectedStageExists =
            isSelectedWorkflow && next.stages.some((stage) => stage.number === previous.stageNumber)
          const shouldClearInspection = isSelectedWorkflow && !inspectedStageExists
          return shouldClearInspection ? null : previous
        })
        setConnection({ workflowId, isReady: true, label: 'Live updates connected' })
        if (movementPhase.current !== 'posting') {
          setPending((previous) => (previous?.workflowId === workflowId ? null : previous))
          movementAdmitted.current = false
          movementPhase.current = 'idle'
        }
      } catch {
        setConnection({ workflowId, isReady: false, label: 'Resynchronizing' })
      }
    })
    return () => {
      isClosed = true
      source.close()
    }
  }, [queryClient, workflowId, revision])

  const isConnected = connection.workflowId === workflowId && connection.isReady
  const pendingMovement = pending?.workflowId === workflowId ? pending : null
  const canSubmitMovement = Boolean(
    workflowId &&
    workflow?.checkpoint &&
    isConnected &&
    !pendingMovement &&
    !workflow.movement_busy &&
    workflow.current_status === 'available',
  )
  const actions = workflow?.current_status === 'available' ? movementActions(workflow) : []
  const forward =
    workflow?.current_status === 'available'
      ? forwardActions(workflow, selectedStageNumber)
      : { next: null, to: null, all: null }

  const refresh = () => {
    setConnection({ workflowId, isReady: false, label: 'Synchronizing' })
    setRevision((value) => value + 1)
  }

  const run = async (action: Action) => {
    if (!canSubmitMovement || !workflowId || !workflow?.checkpoint || movementAdmitted.current) return
    const requestWorkflowId = workflowId
    const requestToken = ++requestNumber.current
    const isCurrentRequest = () =>
      currentWorkflowId.current === requestWorkflowId && requestNumber.current === requestToken
    movementAdmitted.current = true
    movementPhase.current = 'posting'
    setPending({ workflowId: requestWorkflowId, label: action.label })
    setIssue(null)
    try {
      await movementMutation.mutateAsync({
        id: requestWorkflowId,
        choice: action.choice,
        checkpoint: workflow.checkpoint.state,
      })
    } catch (error) {
      if (isCurrentRequest()) {
        const staleCheckpoint = error instanceof ApiFailure && error.code === 'stale_checkpoint'
        const issueMessage = staleCheckpoint
          ? 'Checkpoint changed before the movement. Refreshing the live view; review it before submitting again.'
          : 'Movement response was not confirmed: ' +
            messageFrom(error) +
            '. Check live status before submitting again.'
        setIssue({ workflowId: requestWorkflowId, message: issueMessage })
        setPending(null)
      }
    } finally {
      if (isCurrentRequest()) {
        movementPhase.current = 'reconciling'
        refresh()
      }
    }
  }

  const chooseStage = (stageNumber: number) => {
    if (!workflowId) return
    setInspection({ workflowId, stageNumber })
  }

  return {
    workflowId,
    workflow,
    selectedSummary: workflow?.workflow ?? null,
    selectedStageNumber,
    selectedStage,
    liveLabel: connectionLabel(workflowId, connection),
    isConnected,
    canSubmitMovement,
    pendingMovement,
    movementIssue: issue?.workflowId === workflowId ? issue.message : null,
    actions,
    forward,
    run,
    refresh,
    chooseStage,
  }
}
