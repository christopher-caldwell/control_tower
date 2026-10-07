import { ActionIcon, Group, Modal, Paper, Text } from '@mantine/core'
import { IconMaximize } from '@tabler/icons-react'
import type { FC } from 'react'
import { useState } from 'react'

import type { RoleObservation } from '@/api/types'
import { OutputStream } from '@/features/workflows/stage_inspection/components/output_stream'

type RoleOutputProps = { result: RoleObservation }

export const RoleOutput: FC<RoleOutputProps> = ({ result }) => {
  const [opened, setOpened] = useState(false)
  const label = `Stage ${result.stage.number} · Stage Action ${result.role}`
  return (
    <>
      <Paper withBorder p="sm" mt="sm">
        <Group justify="space-between" wrap="nowrap" align="flex-start">
          <Text fw={600} size="sm">
            {label}
          </Text>
          <ActionIcon
            variant="subtle"
            size="sm"
            aria-label={`Expand output for ${label}`}
            onClick={() => setOpened(true)}
          >
            <IconMaximize size={16} />
          </ActionIcon>
        </Group>
        <RoleOutputContents result={result} />
      </Paper>
      <Modal
        opened={opened}
        onClose={() => setOpened(false)}
        title={label}
        size="min(1100px, 90vw)"
        centered
        closeButtonProps={{ 'aria-label': 'Close expanded output' }}
      >
        <Paper withBorder p="sm">
          <Text fw={600} size="sm">
            {label}
          </Text>
          <RoleOutputContents result={result} expanded />
        </Paper>
      </Modal>
    </>
  )
}

const RoleOutputContents: FC<RoleOutputProps & { expanded?: boolean }> = ({ result, expanded = false }) => {
  if (result.state === 'in_progress') {
    return (
      <Text c="dimmed" size="xs">
        Output will be available when the Stage Action returns.
      </Text>
    )
  }
  if (result.state === 'launch_failed') {
    return (
      <Text c="red.3" size="xs">
        {result.message ?? 'The executable could not be started.'}
      </Text>
    )
  }
  return (
    <>
      <Text c="dimmed" size="xs">
        {result.state === 'succeeded' ? 'Completed' : 'Exit ' + (result.exit_code ?? 'unknown')}
      </Text>
      <OutputStream name="stdout" contents={result.stdout} expanded={expanded} />
      <OutputStream name="stderr" contents={result.stderr} expanded={expanded} />
    </>
  )
}
