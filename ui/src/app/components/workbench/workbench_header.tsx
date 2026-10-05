import { ActionIcon, Badge, Divider, Group, Stack, Text, Tooltip } from '@mantine/core'
import { IconRefresh } from '@tabler/icons-react'
import type { FC } from 'react'

import styles from '@/app/components/workbench/workbench.module.css'

type WorkbenchHeaderProps = {
  workspaceName: string | null
  liveLabel: string
  isConnected: boolean
  workspaceIssue: string | null
  canRefresh: boolean
  onRefresh: () => void
}

export const WorkbenchHeader: FC<WorkbenchHeaderProps> = ({
  workspaceName,
  liveLabel,
  isConnected,
  workspaceIssue,
  canRefresh,
  onRefresh,
}) => {
  const connectionColor = isConnected ? 'green' : 'yellow'
  const connectionText = workspaceIssue ? 'Disconnected' : liveLabel
  return (
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
            {workspaceName ?? 'Connecting'}
          </Text>
        </Stack>
      </Group>
      <Group gap="xs" wrap="nowrap">
        <Badge variant="dot" color={connectionColor}>
          {connectionText}
        </Badge>
        <Tooltip label="Refresh live view">
          <ActionIcon
            variant="subtle"
            color="gray"
            aria-label="Refresh live view"
            onClick={onRefresh}
            disabled={!canRefresh}
          >
            <IconRefresh size={18} />
          </ActionIcon>
        </Tooltip>
      </Group>
    </Group>
  )
}
