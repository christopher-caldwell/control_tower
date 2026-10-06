import {
  ActionIcon,
  Alert,
  Badge,
  Button,
  Group,
  Loader,
  Stack,
  Text,
  TextInput,
  Title,
  Tooltip,
  UnstyledButton,
} from '@mantine/core'
import { IconChevronLeft, IconChevronRight } from '@tabler/icons-react'
import type { FC } from 'react'
import { useState } from 'react'

import styles from '@/features/workspace/workflows/components/workflow_rail.module.css'
import type { WorkspaceWorkflowsModel } from '@/features/workspace/workflows/hooks/use_workspace_workflows'

export type WorkflowRailProps = {
  model: WorkspaceWorkflowsModel
  collapsed: boolean
  onToggleCollapsed: () => void
}

export const WorkflowRail: FC<WorkflowRailProps> = ({ model, collapsed, onToggleCollapsed }) => {
  const workflows = model.workspace?.workflows ?? []
  const [query, setQuery] = useState('')
  const normalizedQuery = query.trim().toLowerCase()
  const matchingWorkflows = normalizedQuery
    ? workflows.filter((workflow) => workflow.name.toLowerCase().includes(normalizedQuery))
    : workflows
  const shouldShowLoading = model.isLoading && !collapsed
  const hasEmptyInventory = model.workspace !== undefined && workflows.length === 0 && model.issue === null
  const shouldShowEmpty = !collapsed && hasEmptyInventory
  const shouldShowIssue = !collapsed && model.issue !== null
  const shouldShowNoMatches =
    !collapsed && !model.isLoading && model.issue === null && workflows.length > 0 && matchingWorkflows.length === 0
  const issueTitle = model.workspace === undefined ? 'Workspace unavailable' : 'Workspace refresh failed'
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
        {!collapsed ? (
          <TextInput
            label="Search workflows"
            placeholder="Search workflows"
            value={query}
            onChange={(event) => setQuery(event.currentTarget.value)}
          />
        ) : null}
        {shouldShowLoading ? (
          <Group>
            <Loader size="xs" />
            <Text size="sm">Discovering workflows…</Text>
          </Group>
        ) : null}
        {matchingWorkflows.map((workflow) => (
          <WorkflowRailItem
            key={workflow.id}
            name={workflow.name}
            isSelected={workflow.id === model.selectedWorkflowId}
            isCollapsed={collapsed}
            onSelect={() => model.selectWorkflow(workflow.id)}
          />
        ))}
        {shouldShowNoMatches ? (
          <Text size="sm" c="dimmed">
            No matching workflows
          </Text>
        ) : null}
        {shouldShowEmpty ? (
          <Text size="sm" c="dimmed">
            No workflow directories were found. Add one under workflows/ and restart the UI.
          </Text>
        ) : null}
        {shouldShowIssue ? (
          <Alert color="red" title={issueTitle}>
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
