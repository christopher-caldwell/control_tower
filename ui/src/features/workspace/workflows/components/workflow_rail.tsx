import {
  ActionIcon,
  Alert,
  Badge,
  Button,
  Group,
  Loader,
  Stack,
  Text,
  Title,
  Tooltip,
  UnstyledButton,
} from '@mantine/core'
import { IconChevronLeft, IconChevronRight } from '@tabler/icons-react'
import type { FC } from 'react'

import styles from '@/features/workspace/workflows/components/workflow_rail.module.css'
import type { WorkspaceWorkflowsModel } from '@/features/workspace/workflows/hooks/use_workspace_workflows'

export type WorkflowRailProps = {
  model: WorkspaceWorkflowsModel
  collapsed: boolean
  onToggleCollapsed: () => void
}

export const WorkflowRail: FC<WorkflowRailProps> = ({ model, collapsed, onToggleCollapsed }) => {
  const workflows = model.workspace?.workflows ?? []
  const shouldShowLoading = model.isLoading && !collapsed
  const shouldShowEmpty = !collapsed && workflows.length === 0 && !model.isLoading
  const shouldShowIssue = !collapsed && model.issue !== null
  return (
    <aside className={styles.rail} aria-label="Workspace workflows">
      <Group justify={collapsed ? 'center' : 'space-between'} className={styles.heading} wrap="nowrap">
        {collapsed ? null : (
          <>
            <Title order={2}>Workflows</Title>
            <Badge color="violet" variant="light">
              {model.workspace?.workflows.length ?? '—'}
            </Badge>
          </>
        )}
        <Tooltip label={collapsed ? 'Expand workflow rail' : 'Collapse workflow rail'}>
          <ActionIcon
            variant="subtle"
            aria-label={collapsed ? 'Expand workflow rail' : 'Collapse workflow rail'}
            onClick={onToggleCollapsed}
          >
            {collapsed ? <IconChevronRight size={18} /> : <IconChevronLeft size={18} />}
          </ActionIcon>
        </Tooltip>
      </Group>
      <Stack gap="xs" className={styles.scroll} p={collapsed ? 8 : 12}>
        {shouldShowLoading ? (
          <Group>
            <Loader size="xs" />
            <Text size="sm">Discovering workflows…</Text>
          </Group>
        ) : null}
        {workflows.map((workflow) => (
          <WorkflowRailItem
            key={workflow.id}
            name={workflow.name}
            isSelected={workflow.id === model.selectedWorkflowId}
            isCollapsed={collapsed}
            onSelect={() => model.selectWorkflow(workflow.id)}
          />
        ))}
        {shouldShowEmpty ? (
          <Text size="sm" c="dimmed">
            No workflow directories were found. Add one under workflows/ and restart the UI.
          </Text>
        ) : null}
        {shouldShowIssue ? (
          <Alert color="red" title="Workspace unavailable">
            <Text size="sm">{model.issue}</Text>
            <Button size="compact-sm" variant="light" mt="xs" onClick={model.retry}>
              Retry workspace
            </Button>
          </Alert>
        ) : null}
      </Stack>
    </aside>
  )
}

type WorkflowRailItemProps = {
  name: string
  isSelected: boolean
  isCollapsed: boolean
  onSelect: () => void
}

const WorkflowRailItem: FC<WorkflowRailItemProps> = ({ name, isSelected, isCollapsed, onSelect }) => {
  if (isCollapsed) {
    return (
      <Tooltip label={name} position="right">
        <ActionIcon
          size="lg"
          variant={isSelected ? 'filled' : 'subtle'}
          bg={isSelected ? 'var(--mantine-other-selection)' : undefined}
          c={isSelected ? 'var(--mantine-other-navigation)' : undefined}
          aria-label={'Select workflow ' + name}
          aria-pressed={isSelected}
          onClick={onSelect}
        >
          {name.slice(0, 1).toUpperCase()}
        </ActionIcon>
      </Tooltip>
    )
  }
  return (
    <UnstyledButton
      className={styles.workflow}
      data-selected={isSelected}
      aria-current={isSelected ? 'page' : undefined}
      onClick={onSelect}
    >
      <Text size="sm" fw={600}>
        {name}
      </Text>
    </UnstyledButton>
  )
}
