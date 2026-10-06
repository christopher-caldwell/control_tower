import type { CSSProperties, FC, KeyboardEvent, PointerEvent } from 'react'
import { useCallback, useEffect, useRef, useState } from 'react'

import { unavailableMovementLabel } from '@/app/components/workbench/unavailable_movement_label'
import styles from '@/app/components/workbench/workbench.module.css'
import { WorkbenchHeader } from '@/app/components/workbench/workbench_header'
import { useWorkflowExecution, WorkflowExecution } from '@/features/workflows/execution'
import { StageInspector, useStageInspection } from '@/features/workflows/stage_inspection'
import { useWorkspaceWorkflows, WorkflowRail } from '@/features/workspace/workflows'

const inspectorWidthStorageKey = 'control-tower-stage-inspector-width-v1'
const defaultInspectorWidth = 382
const minimumInspectorWidth = 300
const centerMinimumWidth = 520

export const Workbench: FC = () => {
  const workflows = useWorkspaceWorkflows()
  const execution = useWorkflowExecution({ workflowId: workflows.selectedWorkflowId })
  const inspection = useStageInspection({
    workflowId: execution.workflowId,
    stage: execution.selectedStage,
    checkpoint: execution.workflow?.checkpoint ?? null,
    observation: execution.workflow?.observation ?? null,
  })
  const [isWorkflowRailCollapsed, setWorkflowRailCollapsed] = useState(false)
  const [isInspectorCollapsed, setInspectorCollapsed] = useState(false)
  const [viewportWidth, setViewportWidth] = useState(() => window.innerWidth)
  const [preferredInspectorWidth, setPreferredInspectorWidth] = useState(() => readInspectorWidth())
  const resizeStart = useRef<{ pointerId: number; clientX: number; width: number } | null>(null)
  const workflowRailWidth = isWorkflowRailCollapsed ? 60 : 252
  const maximumInspectorWidth = Math.max(minimumInspectorWidth, viewportWidth - workflowRailWidth - centerMinimumWidth)
  const renderedInspectorWidth = clamp(preferredInspectorWidth, minimumInspectorWidth, maximumInspectorWidth)

  useEffect(() => {
    const handleResize = () => setViewportWidth(window.innerWidth)
    window.addEventListener('resize', handleResize)
    return () => window.removeEventListener('resize', handleResize)
  }, [])

  const updateInspectorWidth = useCallback(
    (nextWidth: number) => {
      const leftWidth = isWorkflowRailCollapsed ? 60 : 252
      const maxWidth = Math.max(minimumInspectorWidth, window.innerWidth - leftWidth - centerMinimumWidth)
      const width = clamp(nextWidth, minimumInspectorWidth, maxWidth)
      setPreferredInspectorWidth(width)
      try {
        window.localStorage.setItem(inspectorWidthStorageKey, String(width))
      } catch {
        // Storage can be unavailable in restricted browser contexts; resizing still works for this page.
      }
    },
    [isWorkflowRailCollapsed],
  )

  const handleResizeKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return
    event.preventDefault()
    updateInspectorWidth(renderedInspectorWidth + (event.key === 'ArrowLeft' ? 16 : -16))
  }

  const handlePointerDown = (event: PointerEvent<HTMLDivElement>) => {
    event.preventDefault()
    event.currentTarget.setPointerCapture(event.pointerId)
    resizeStart.current = { pointerId: event.pointerId, clientX: event.clientX, width: renderedInspectorWidth }
  }
  const handlePointerMove = (event: PointerEvent<HTMLDivElement>) => {
    if (resizeStart.current?.pointerId !== event.pointerId) return
    updateInspectorWidth(resizeStart.current.width + resizeStart.current.clientX - event.clientX)
  }
  const handlePointerEnd = (event: PointerEvent<HTMLDivElement>) => {
    if (resizeStart.current?.pointerId === event.pointerId) resizeStart.current = null
  }
  const shouldShowEmptyState =
    workflows.workspace !== undefined && workflows.issue === null && workflows.workspace.workflows.length === 0
  const emptyStateMessage = shouldShowEmptyState
    ? 'Add a workflow directory under workflows/ and restart the UI.'
    : null
  const movementLabel = unavailableMovementLabel(workflows, execution)
  const shouldDisableRefresh = workflows.selectedWorkflowId === null || execution.pendingMovement !== null

  return (
    <div className={styles.frame}>
      <WorkbenchHeader
        workspaceName={workflows.workspace?.name ?? null}
        liveLabel={execution.liveLabel}
        isConnected={execution.isConnected}
        workspaceIssue={workflows.issue}
        canRefresh={!shouldDisableRefresh}
        onRefresh={execution.refresh}
      />
      <div
        className={getGridClassName(isWorkflowRailCollapsed, isInspectorCollapsed)}
        style={{ '--inspector-width': `${renderedInspectorWidth}px` } as CSSProperties}
      >
        <WorkflowRail
          model={workflows}
          collapsed={isWorkflowRailCollapsed}
          onToggleCollapsed={() => setWorkflowRailCollapsed((isCollapsed) => !isCollapsed)}
        />
        <WorkflowExecution
          model={execution}
          selectedWorkflow={workflows.selectedWorkflow}
          emptyStateMessage={emptyStateMessage}
          unavailableLabel={movementLabel}
        />
        <StageInspector
          model={inspection}
          open={!isInspectorCollapsed}
          onCollapse={() => setInspectorCollapsed(true)}
          onExpand={() => setInspectorCollapsed(false)}
          width={renderedInspectorWidth}
          minWidth={minimumInspectorWidth}
          maxWidth={maximumInspectorWidth}
          onResizeKeyDown={handleResizeKeyDown}
          onResizePointerDown={handlePointerDown}
          onResizePointerMove={handlePointerMove}
          onResizePointerEnd={handlePointerEnd}
        />
      </div>
    </div>
  )
}

const readInspectorWidth = (): number => {
  try {
    const stored = window.localStorage.getItem(inspectorWidthStorageKey)
    if (stored === null || stored.trim() === '') return defaultInspectorWidth
    const value = Number(stored)
    return Number.isFinite(value) && value > 0 ? value : defaultInspectorWidth
  } catch {
    return defaultInspectorWidth
  }
}

const clamp = (value: number, minimum: number, maximum: number): number => Math.min(maximum, Math.max(minimum, value))

const getGridClassName = (isWorkflowRailCollapsed: boolean, isInspectorCollapsed: boolean): string => {
  const classNames = [styles.grid]
  if (isWorkflowRailCollapsed) classNames.push(styles.leftCollapsed)
  if (isInspectorCollapsed) classNames.push(styles.rightCollapsed)
  return classNames.join(' ')
}
