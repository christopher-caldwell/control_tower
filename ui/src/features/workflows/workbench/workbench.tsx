import {
  ActionIcon,
  Alert,
  Badge,
  Button,
  Divider,
  Group,
  Loader,
  Stack,
  Text,
  Title,
  Tooltip,
  UnstyledButton,
} from '@mantine/core'
import { IconArrowBackUp, IconArrowRight, IconChevronLeft, IconChevronRight, IconRefresh } from '@tabler/icons-react'
import type { FC } from 'react'
import { useState } from 'react'

import { Inspector } from '@/features/workflows/workbench/components/inspector'
import type { Action } from '@/features/workflows/workbench/presentation'
import { checkpointPosition } from '@/features/workflows/workbench/presentation'
import { useWorkbench } from '@/features/workflows/workbench/use_workbench'
import styles from '@/features/workflows/workbench/workbench.module.css'

export const Workbench: FC = () => {
  const model = useWorkbench()
  const [leftCollapsed, setLeftCollapsed] = useState(false)
  const [rightCollapsed, setRightCollapsed] = useState(false)
  const { workspace, workflow, actions, forward, busy, pending } = model
  const primary = forward.next ?? actions[0]
  const alternate = forward.next ? actions.find((action) => action.choice.direction === 'down') : actions[1]
  const finalAccepted =
    workflow?.checkpoint?.accepted_stage?.number === workflow?.stages.at(-1)?.number &&
    Boolean(workflow?.checkpoint?.accepted_stage) &&
    !workflow?.checkpoint?.pending_transition
  const inactive = !model.workflowId
    ? 'Select a workflow'
    : model.workspaceIssue
      ? 'Workspace unavailable'
      : !workflow
        ? 'Connecting to workflow'
        : workflow.current_status === 'unavailable'
          ? 'Workflow unavailable'
          : finalAccepted
            ? 'All stages applied'
            : 'No immediate movement available'
  const movementButton = (action: Action, secondary = false) => (
    <Tooltip key={action.label} label={action.label} disabled={action.label.length <= 60} multiline maw={360}>
      <Button
        size="sm"
        maw="100%"
        variant={secondary ? 'default' : action === forward.next ? 'subtle' : 'filled'}
        loading={pending?.label === action.label}
        disabled={busy || !workflow?.checkpoint}
        onClick={() => void model.run(action)}
        rightSection={secondary ? <IconArrowBackUp size={16} /> : <IconArrowRight size={16} />}
      >
        {action.label}
      </Button>
    </Tooltip>
  )
  return (
    <div className={styles.frame}>
      <Group className={styles.header} justify="space-between" wrap="nowrap">
        <Group gap="lg" wrap="nowrap" style={{ flex: 1, minWidth: 0 }}>
          <Stack gap={0}>
            <Text fw={700}>Control Tower</Text>
            <Text size="xs" c="violet.2">
              LOCAL WORKBENCH
            </Text>
          </Stack>
          <Divider orientation="vertical" />
          <Stack gap={0} style={{ flex: 1, minWidth: 0 }}>
            <Text size="xs" c="violet.2">
              WORKSPACE
            </Text>
            <Text fw={600} truncate>
              {workspace?.name ?? 'Connecting'}
            </Text>
          </Stack>
        </Group>
        <Group gap="xs" wrap="nowrap">
          <Badge variant="dot" color={model.ready ? 'green' : 'yellow'}>
            {model.workspaceIssue ? 'Disconnected' : model.live}
          </Badge>
          <Tooltip label="Refresh live view">
            <ActionIcon
              variant="subtle"
              color="gray"
              aria-label="Refresh live view"
              onClick={model.refresh}
              disabled={!model.workflowId || Boolean(pending)}
            >
              <IconRefresh size={18} />
            </ActionIcon>
          </Tooltip>
        </Group>
      </Group>
      <div
        className={[styles.grid, leftCollapsed && styles.leftCollapsed, rightCollapsed && styles.rightCollapsed]
          .filter(Boolean)
          .join(' ')}
      >
        <aside className={styles.rail} aria-label="Workspace workflows">
          <Group justify={leftCollapsed ? 'center' : 'space-between'} className={styles.paneHeading} wrap="nowrap">
            {!leftCollapsed && (
              <>
                <Title order={2}>Workflows</Title>
                <Badge color="violet" variant="light">
                  {workspace?.workflows.length ?? '—'}
                </Badge>
              </>
            )}
            <Tooltip label={leftCollapsed ? 'Expand workflow rail' : 'Collapse workflow rail'}>
              <ActionIcon
                variant="subtle"
                aria-label={leftCollapsed ? 'Expand workflow rail' : 'Collapse workflow rail'}
                onClick={() => setLeftCollapsed((value) => !value)}
              >
                {leftCollapsed ? <IconChevronRight size={18} /> : <IconChevronLeft size={18} />}
              </ActionIcon>
            </Tooltip>
          </Group>
          <Stack gap="xs" className={styles.scroll} p={leftCollapsed ? 8 : 12}>
            {model.workspaceLoading && !leftCollapsed && (
              <Group>
                <Loader size="xs" />
                <Text size="sm">Discovering workflows…</Text>
              </Group>
            )}
            {workspace?.workflows.map((item) =>
              leftCollapsed ? (
                <Tooltip key={item.id} label={item.name} position="right">
                  <ActionIcon
                    size="lg"
                    variant={item.id === model.workflowId ? 'filled' : 'subtle'}
                    bg={item.id === model.workflowId ? 'var(--mantine-other-selection)' : undefined}
                    c={item.id === model.workflowId ? 'var(--mantine-other-navigation)' : undefined}
                    aria-label={`Select workflow ${item.name}`}
                    aria-pressed={item.id === model.workflowId}
                    onClick={() => model.selectWorkflow(item.id)}
                  >
                    {item.name.slice(0, 1).toUpperCase()}
                  </ActionIcon>
                </Tooltip>
              ) : (
                <UnstyledButton
                  key={item.id}
                  className={styles.workflow}
                  data-selected={item.id === model.workflowId}
                  aria-current={item.id === model.workflowId ? 'page' : undefined}
                  onClick={() => model.selectWorkflow(item.id)}
                >
                  <Text size="sm" fw={600}>
                    {item.name}
                  </Text>
                </UnstyledButton>
              ),
            )}
            {!leftCollapsed && workspace?.workflows.length === 0 && (
              <Text size="sm" c="dimmed">
                No workflow directories were found. Add one under workflows/ and restart the UI.
              </Text>
            )}
            {!leftCollapsed && model.workspaceIssue && (
              <Alert color="red" title="Workspace unavailable">
                <Text size="sm">{model.workspaceIssue}</Text>
                <Button size="compact-sm" variant="light" mt="xs" onClick={model.retryWorkspace}>
                  Retry workspace
                </Button>
              </Alert>
            )}
          </Stack>
        </aside>
        <main className={styles.center} aria-label="Ordered stages">
          <div className={styles.stageScroll}>
            <Group justify="space-between" align="flex-start" mb="xl">
              <Stack gap={4} style={{ flex: '1 1 180px', minWidth: 0 }}>
                <Text size="xs" c="dimmed">
                  WORKFLOW / {model.selectedSummary?.id ?? '—'}
                </Text>
                <Title order={1} className={styles.longText}>
                  {model.selectedSummary?.name ?? 'Select a workflow'}
                </Title>
                <Text size="sm" c="dimmed">
                  Ordered stages and current position.
                </Text>
              </Stack>
              {workflow?.checkpoint && (
                <Stack gap={6} align="flex-end" style={{ maxWidth: '100%' }}>
                  <Text size="xs" c="dimmed">
                    CURRENT CHECKPOINT
                  </Text>
                  <Text size="sm" fw={600} className={styles.longText}>
                    {checkpointPosition(workflow.checkpoint)}
                  </Text>
                  {workflow.checkpoint.pending_transition && (
                    <Badge color="yellow">
                      Pending {workflow.checkpoint.pending_transition.direction} · Stage{' '}
                      {workflow.checkpoint.pending_transition.stage.number}
                    </Badge>
                  )}
                  {workflow.movement_busy && (
                    <Badge color="teal">
                      Movement active
                      {workflow.observation?.active_role ? ` · ${workflow.observation.active_role.role}` : ''}
                    </Badge>
                  )}
                </Stack>
              )}
            </Group>
            {workflow?.status_issue && (
              <Alert color="red" title="Current checkpoint unavailable" mb="md">
                {workflow.status_issue}
                <Text size="xs" mt="xs">
                  Movement is disabled until current status is readable.
                </Text>
              </Alert>
            )}
            {workflow?.observation?.state === 'stopped' && workflow.observation.failure && (
              <Alert color="red" title="Movement stopped" mb="md">
                {workflow.observation.failure.message}
              </Alert>
            )}
            {!model.ready && model.workflowId && (
              <Group mb="md" gap="xs">
                <Loader size="xs" />
                <Text size="sm" c="dimmed">
                  {model.live === 'Reconnecting'
                    ? 'Connection lost. Reconnecting to the live view…'
                    : 'Waiting for a fresh workflow snapshot…'}
                </Text>
                <Button size="compact-xs" variant="subtle" onClick={model.refresh}>
                  Refresh workflow
                </Button>
              </Group>
            )}
            {workflow?.checkpoint && (
              <>
                <Group justify="space-between" mb="sm">
                  <Title order={2}>Stages</Title>
                  <Text size="xs" c="dimmed">
                    {workflow.stages.length} {workflow.stages.length === 1 ? 'stage' : 'stages'}
                  </Text>
                </Group>
                {workflow.stages.map((stage) => (
                  <UnstyledButton
                    key={stage.number}
                    className={styles.stage}
                    data-selected={stage.number === model.stageNumber}
                    aria-pressed={stage.number === model.stageNumber}
                    onClick={() => model.chooseStage(stage.number)}
                  >
                    <Group wrap="nowrap" align="flex-start">
                      <Badge
                        color={stage.state === 'accepted' ? 'green' : stage.state === 'pending' ? 'yellow' : 'gray'}
                        variant="light"
                        h={32}
                        miw={40}
                        radius="xl"
                      >
                        <Text size="xs" fw={700}>
                          {stage.number}
                        </Text>
                      </Badge>
                      <Stack gap={4} className={styles.stageCopy}>
                        <Group gap={6}>
                          <Text size="xs" c={stage.state === 'pending' ? 'yellow.3' : 'dimmed'}>
                            {stage.state === 'accepted'
                              ? 'Applied'
                              : stage.state === 'pending'
                                ? 'Pending transition'
                                : 'Not applied'}
                          </Text>
                          {stage.is_accepted_checkpoint && (
                            <Badge color="green" size="xs">
                              Checkpoint
                            </Badge>
                          )}
                          {workflow.observation?.active_role?.stage.number === stage.number && (
                            <Badge color="teal" size="xs">
                              Running {workflow.observation.active_role.role}
                            </Badge>
                          )}
                        </Group>
                        <Text fw={600}>{stage.name}</Text>
                        <Text size="xs" c="dimmed">
                          {stage.definitions.map((role) => role.role).join(' · ')}
                        </Text>
                      </Stack>
                      <IconChevronRight size={18} />
                    </Group>
                  </UnstyledButton>
                ))}
              </>
            )}
            {!model.workflowId && !model.workspaceLoading && (
              <Stack align="center" mt="xl">
                <Title order={2}>{model.workspaceIssue ? 'Workspace unavailable' : 'No workflows found'}</Title>
                <Text c="dimmed" ta="center">
                  {model.workspaceIssue ?? 'Add a workflow directory under workflows/ and restart the UI.'}
                </Text>
              </Stack>
            )}
          </div>
          <Stack gap="sm" className={styles.dock}>
            {model.movementIssue && (
              <Alert color="red" role="status">
                {model.movementIssue}
              </Alert>
            )}
            <Group justify="flex-end" gap="sm">
              {alternate && movementButton(alternate, true)}
              {forward.next && movementButton(forward.next)}
              {forward.to && movementButton(forward.to)}
              {forward.all && movementButton(forward.all)}
              {!forward.next && primary && movementButton(primary)}
              {!primary && <Button disabled>{inactive}</Button>}
            </Group>
          </Stack>
        </main>
        <aside className={styles.inspector} aria-label="Selected stage inspector">
          {rightCollapsed && (
            <UnstyledButton
              className={styles.reopen}
              aria-label="Expand stage inspector"
              onClick={() => setRightCollapsed(false)}
            >
              <IconChevronLeft size={18} />
              <Text size="xs" className={styles.reopenText}>
                INSPECTOR
              </Text>
            </UnstyledButton>
          )}
          <Inspector
            open={!rightCollapsed}
            stage={model.selectedStage}
            checkpoint={workflow?.checkpoint ?? null}
            observation={workflow?.observation ?? null}
            definition={model.definition}
            definitionIssue={model.definitionIssue}
            definitionLoading={model.definitionLoading}
            retryDefinition={model.retryDefinition}
            onCollapse={() => setRightCollapsed(true)}
          />
        </aside>
      </div>
    </div>
  )
}
