import { Paper, Text } from '@mantine/core'
import type { FC } from 'react'

import type { RoleObservation } from '@/api/types'
import { OutputStream } from '@/features/workflows/stage_inspection/components/output_stream'

type RoleOutputProps = { result: RoleObservation }

export const RoleOutput: FC<RoleOutputProps> = ({ result }) => {
  if (result.state === 'in_progress') {
    return (
      <Paper withBorder p="sm" mt="sm">
        <RoleHeading result={result} />
        <Text c="dimmed" size="xs">
          Output will be available when the role returns.
        </Text>
      </Paper>
    )
  }
  if (result.state === 'launch_failed') {
    return (
      <Paper withBorder p="sm" mt="sm">
        <RoleHeading result={result} />
        <Text c="red.3" size="xs">
          {result.message ?? 'The executable could not be started.'}
        </Text>
      </Paper>
    )
  }
  return (
    <Paper withBorder p="sm" mt="sm">
      <RoleHeading result={result} />
      <Text c="dimmed" size="xs">
        {result.state === 'succeeded' ? 'Completed' : 'Exit ' + (result.exit_code ?? 'unknown')}
      </Text>
      <OutputStream name="stdout" contents={result.stdout} />
      <OutputStream name="stderr" contents={result.stderr} />
    </Paper>
  )
}

const RoleHeading: FC<RoleOutputProps> = ({ result }) => (
  <Text fw={600} size="sm">
    Stage {result.stage.number} · {result.role}
  </Text>
)
