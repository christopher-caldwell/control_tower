import { ActionIcon, Group, Stack, Text, Title, Tooltip } from '@mantine/core'
import { IconChevronLeft, IconChevronRight } from '@tabler/icons-react'
import type { FC, KeyboardEvent, PointerEvent } from 'react'

import { RoleOutput } from '@/features/workflows/stage_inspection/components/role_output'
import { StageDetails } from '@/features/workflows/stage_inspection/components/stage_details'
import styles from '@/features/workflows/stage_inspection/components/stage_inspector.module.css'
import type { StageInspectionModel } from '@/features/workflows/stage_inspection/hooks/use_stage_inspection'

export type StageInspectorProps = {
  model: StageInspectionModel
  open: boolean
  onCollapse: () => void
  onExpand: () => void
  width: number
  minWidth: number
  maxWidth: number
  onResizeKeyDown: (event: KeyboardEvent<HTMLDivElement>) => void
  onResizePointerDown: (event: PointerEvent<HTMLDivElement>) => void
  onResizePointerMove: (event: PointerEvent<HTMLDivElement>) => void
  onResizePointerEnd: (event: PointerEvent<HTMLDivElement>) => void
}

export const StageInspector: FC<StageInspectorProps> = ({
  model,
  open,
  onCollapse,
  onExpand,
  width,
  minWidth,
  maxWidth,
  onResizeKeyDown,
  onResizePointerDown,
  onResizePointerMove,
  onResizePointerEnd,
}) => {
  const hasRetainedOutput = Boolean(model.retainedObservation)
  const hasSelectedStage = model.stage !== null
  const showEmptySelection = !hasSelectedStage && !hasRetainedOutput
  return (
    <aside className={styles.inspector} aria-label="Selected stage inspector">
      {open ? (
        <div
          className={styles.separator}
          role="separator"
          aria-label="Resize stage inspector"
          aria-controls="stage-inspector-pane"
          aria-orientation="vertical"
          aria-valuenow={Math.round(width)}
          aria-valuemin={Math.round(minWidth)}
          aria-valuemax={Math.round(maxWidth)}
          tabIndex={0}
          onKeyDown={onResizeKeyDown}
          onPointerDown={onResizePointerDown}
          onPointerMove={onResizePointerMove}
          onPointerUp={onResizePointerEnd}
          onPointerCancel={onResizePointerEnd}
        />
      ) : null}
      <Stack
        className={styles.panel}
        id="stage-inspector-pane"
        gap={0}
        style={{ visibility: open ? 'visible' : 'hidden' }}
        inert={!open}
      >
        <Group justify="space-between" className={styles.heading} wrap="nowrap">
          <Title order={2}>Stage inspector</Title>
          <Tooltip label="Collapse stage inspector">
            <ActionIcon variant="subtle" aria-label="Collapse stage inspector" onClick={onCollapse}>
              <IconChevronRight size={18} />
            </ActionIcon>
          </Tooltip>
        </Group>
        <div className={styles.scroll} role="region" aria-label="Stage inspector content">
          {showEmptySelection ? <EmptySelection /> : null}
          {model.retainedObservation ? <RetainedOutput observation={model.retainedObservation} /> : null}
          <StageDetails model={model} />
        </div>
      </Stack>
      {!open ? (
        <Stack className={styles.reopen} gap={0}>
          <Tooltip label="Expand stage inspector">
            <ActionIcon variant="subtle" aria-label="Expand stage inspector" onClick={onExpand}>
              <IconChevronLeft size={18} />
            </ActionIcon>
          </Tooltip>
          <Text size="xs" className={styles.reopenText}>
            INSPECTOR
          </Text>
        </Stack>
      ) : null}
    </aside>
  )
}

const EmptySelection: FC = () => (
  <Stack align="center" mt="xl" p="md">
    <Text fw={600}>Select a stage</Text>
    <Text size="sm" c="dimmed" ta="center">
      Selection only inspects a definition; it never runs a script.
    </Text>
  </Stack>
)

type RetainedOutputProps = { observation: NonNullable<StageInspectionModel['retainedObservation']> }
const RetainedOutput: FC<RetainedOutputProps> = ({ observation }) => (
  <Stack gap={0} p="md">
    <Title order={4}>Latest observed output</Title>
    <Text size="xs" c="dimmed" mt="xs">
      Current stage status is unavailable. These are the latest Stage Action results this host observed.
    </Text>
    {observation.role_results.map((result, index) => (
      <RoleOutput key={index} result={result} />
    ))}
  </Stack>
)
