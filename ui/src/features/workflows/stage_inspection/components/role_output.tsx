import { Group, Paper, Text } from '@mantine/core'
import type { FC } from 'react'

import type { MovementObservation, RoleObservation, StageView } from '@/api/types'
import { OutputStream } from '@/features/workflows/stage_inspection/components/output_stream'
import styles from '@/features/workflows/stage_inspection/components/stage_inspector.module.css'

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

export const RolePreview: FC<RolePreviewProps> = ({ role, stage, results, failure, pendingDirection }) => {
  const isConfigured = stage.definitions.some((definition) => definition.role === role)
  const result = results.filter((item) => item.role === role).at(-1)
  const isFailedRole = failure?.stage?.number === stage.number && failure.role === role
  const roleDirection = role === 'up' || role === 'verify-up' ? 'up' : 'down'
  const historyStatus = getMissingHistoryStatus(stage.state, role, pendingDirection, roleDirection, isConfigured)
  const status = getRoleStatus(result, isFailedRole, historyStatus)
  const hasError = isFailedRole || result?.state === 'failed' || result?.state === 'launch_failed'
  const isSuccessful = result?.state === 'succeeded'
  let color = 'dimmed'
  if (hasError) color = 'red.3'
  if (isSuccessful) color = 'green.3'

  return <StackRole role={role} status={status} color={color} message={result?.message ?? null} />
}

type RolePreviewProps = {
  role: string
  stage: StageView
  results: RoleObservation[]
  failure: MovementObservation['failure']
  pendingDirection?: 'up' | 'down'
}

const getMissingHistoryStatus = (
  stageState: RolePreviewProps['stage']['state'],
  role: string,
  pendingDirection: RolePreviewProps['pendingDirection'],
  roleDirection: 'up' | 'down',
  isConfigured: boolean,
): string => {
  if (!isConfigured) return role.startsWith('verify-') ? 'Not configured' : 'Missing'
  if (stageState === 'accepted') return role === 'up' ? 'Applied · history unavailable' : 'No recorded result'
  if (stageState === 'pending')
    return pendingDirection === roleDirection ? 'Prior outcome unavailable' : 'No recorded result'
  return 'Not attempted'
}

const getRoleStatus = (result: RoleObservation | undefined, isFailedRole: boolean, fallback: string): string => {
  if (result?.state === 'succeeded') return 'Process OK'
  if (result?.state === 'in_progress') return 'In progress'
  if (result?.state === 'failed') return 'Exit ' + (result.exit_code ?? 'unknown')
  if (result?.state === 'launch_failed') return 'Launch failed'
  if (isFailedRole) return 'Failed'
  return fallback
}

type StackRoleProps = { role: string; status: string; color: string; message: string | null }
const StackRole: FC<StackRoleProps> = ({ role, status, color, message }) => (
  <div className={styles.role} aria-label={role + ' result'}>
    <Group justify="space-between" wrap="wrap">
      <Text ff="monospace" size="xs" fw={600}>
        {role}
      </Text>
      <Text size="xs" c={color}>
        {status}
      </Text>
    </Group>
    {message ? (
      <Text size="xs" className={styles.longText}>
        {message}
      </Text>
    ) : null}
  </div>
)
