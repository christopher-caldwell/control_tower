import {
  Accordion,
  ActionIcon,
  Alert,
  Badge,
  Button,
  Group,
  Loader,
  Paper,
  Stack,
  Text,
  Title,
  Tooltip,
} from '@mantine/core'
import { IconChevronRight } from '@tabler/icons-react'
import type { FC } from 'react'

import type { CheckpointView, DefinitionView, MovementObservation, RoleObservation, StageView } from '@/api/types'
import styles from '@/features/workflows/workbench/workbench.module.css'

const RoleOutput: FC<RoleOutputProps> = ({ result }) => (
  <Paper withBorder p="sm" mt="sm">
    <Text fw={600} size="sm">
      Stage {result.stage.number} · {result.role}
    </Text>
    {result.state === 'in_progress' ? (
      <Text c="dimmed" size="xs">
        Output will be available when the role returns.
      </Text>
    ) : result.state === 'launch_failed' ? (
      <Text c="red.3" size="xs">
        {result.message ?? 'The executable could not be started.'}
      </Text>
    ) : (
      <>
        <Text c="dimmed" size="xs">
          {result.state === 'succeeded' ? 'Completed' : `Exit ${result.exit_code ?? 'unknown'}`}
        </Text>
        {(['stdout', 'stderr'] as const).map((stream) => (
          <div key={stream}>
            <Text size="xs" c={stream === 'stderr' ? 'orange.3' : 'teal.3'} mt="sm">
              {stream}
            </Text>
            {result[stream] ? (
              <pre className={styles.code} aria-label={stream}>
                {result[stream]}
              </pre>
            ) : (
              <Text size="xs" c="dimmed">
                Empty stream
              </Text>
            )}
          </div>
        ))}
      </>
    )}
  </Paper>
)
type RoleOutputProps = { result: RoleObservation }

const RolePreview: FC<RolePreviewProps> = ({ role, stage, results, failure, pendingDirection }) => {
  const configured = stage.definitions.some((item) => item.role === role)
  const result = results.filter((item) => item.role === role).at(-1)
  const failed = failure?.stage?.number === stage.number && failure.role === role
  const direction = role === 'up' || role === 'verify-up' ? 'up' : 'down'
  const unknown =
    stage.state === 'accepted'
      ? role === 'up'
        ? 'Applied · history unavailable'
        : 'No recorded result'
      : stage.state === 'pending'
        ? pendingDirection === direction
          ? 'Prior outcome unavailable'
          : 'No recorded result'
        : 'Not attempted'
  const status = result
    ? result.state === 'succeeded'
      ? 'Process OK'
      : result.state === 'in_progress'
        ? 'In progress'
        : result.state === 'failed'
          ? `Exit ${result.exit_code ?? 'unknown'}`
          : 'Launch failed'
    : failed
      ? 'Failed'
      : configured
        ? unknown
        : role.startsWith('verify-')
          ? 'Not configured'
          : 'Missing'
  return (
    <Stack gap={4} className={styles.role} aria-label={`${role} result`}>
      <Group justify="space-between" wrap="wrap">
        <Text ff="monospace" size="xs" fw={600}>
          {role}
        </Text>
        <Text
          size="xs"
          c={
            failed || result?.state === 'failed' || result?.state === 'launch_failed'
              ? 'red.3'
              : result?.state === 'succeeded'
                ? 'green.3'
                : 'dimmed'
          }
        >
          {status}
        </Text>
      </Group>
      {result?.message && (
        <Text size="xs" className={styles.longText}>
          {result.message}
        </Text>
      )}
    </Stack>
  )
}
type RolePreviewProps = {
  role: string
  stage: StageView
  results: RoleObservation[]
  failure: MovementObservation['failure']
  pendingDirection?: 'up' | 'down'
}

export const Inspector: FC<InspectorProps> = ({
  open,
  stage,
  definition,
  definitionIssue,
  definitionLoading,
  checkpoint,
  observation,
  onCollapse,
  retryDefinition,
}) => {
  const retained = !stage && observation?.role_results.length ? observation : null
  const results = stage ? (observation?.role_results.filter((item) => item.stage.number === stage.number) ?? []) : []
  const failure =
    stage && observation?.failure && (observation.failure.stage?.number === stage.number || results.length > 0)
      ? observation.failure
      : null
  return (
    <Stack
      pos="absolute"
      top={0}
      left={0}
      w={381}
      h="100%"
      mih={0}
      gap={0}
      style={{ visibility: open ? 'visible' : 'hidden' }}
      inert={!open}
    >
      <Group justify="space-between" className={styles.paneHeading} wrap="nowrap">
        <Title order={2}>Stage inspector</Title>
        <Tooltip label="Collapse stage inspector">
          <ActionIcon variant="subtle" aria-label="Collapse stage inspector" onClick={onCollapse}>
            <IconChevronRight size={18} />
          </ActionIcon>
        </Tooltip>
      </Group>
      <div className={styles.inspectorScroll} role="region" aria-label="Stage inspector content">
        {!stage && !retained && (
          <Stack align="center" mt="xl" p="md">
            <Text fw={600}>Select a stage</Text>
            <Text size="sm" c="dimmed" ta="center">
              Selection only inspects a definition; it never runs a script.
            </Text>
          </Stack>
        )}
        {retained && (
          <Stack gap={0} p="md">
            <Title order={4}>Latest observed output</Title>
            <Text size="xs" c="dimmed" mt="xs">
              Current stage status is unavailable. These are the latest role results this host observed.
            </Text>
            {retained.role_results.map((result, index) => (
              <RoleOutput key={index} result={result} />
            ))}
          </Stack>
        )}
        {stage && (
          <>
            <Stack gap={0} p="md">
              <Text size="xs" c="dimmed">
                STAGE {stage.number}
              </Text>
              <Title order={3} mt={4} className={styles.longText}>
                {stage.name}
              </Title>
              <Badge
                w="fit-content"
                mt="sm"
                color={stage.state === 'accepted' ? 'green' : stage.state === 'pending' ? 'yellow' : 'gray'}
              >
                {stage.state === 'accepted' ? 'Applied' : stage.state === 'pending' ? 'Pending' : 'Future stage'}
              </Badge>
            </Stack>
            {failure && (
              <Alert
                color="red"
                m="md"
                title={failure.kind === 'checkpoint_save_failed' ? 'Checkpoint was not confirmed' : 'Observed failure'}
              >
                {failure.message}
              </Alert>
            )}
            <Accordion multiple defaultValue={['state', 'mutation', 'verification', 'output', 'definitions']}>
              <Accordion.Item value="state">
                <Accordion.Control>Checkpoint state</Accordion.Control>
                <Accordion.Panel>
                  <Text size="sm">
                    {stage.state === 'accepted'
                      ? 'Applied'
                      : stage.state === 'pending'
                        ? `Pending ${checkpoint?.pending_transition?.direction ?? ''}`
                        : 'Not applied'}
                  </Text>
                  <Text size="xs" c="dimmed">
                    {stage.is_accepted_checkpoint ? 'Last confirmed accepted position' : 'Current workflow state'}
                  </Text>
                </Accordion.Panel>
              </Accordion.Item>
              {(['mutation', 'verification'] as const).map((section) => (
                <Accordion.Item key={section} value={section}>
                  <Accordion.Control>{section === 'mutation' ? 'Mutation' : 'Verification'}</Accordion.Control>
                  <Accordion.Panel>
                    {(section === 'mutation' ? ['up', 'down'] : ['verify-up', 'verify-down']).map((role) => (
                      <RolePreview
                        key={role}
                        role={role}
                        stage={stage}
                        results={results}
                        failure={failure}
                        pendingDirection={checkpoint?.pending_transition?.direction}
                      />
                    ))}
                  </Accordion.Panel>
                </Accordion.Item>
              ))}
              <Accordion.Item value="output">
                <Accordion.Control>Captured output</Accordion.Control>
                <Accordion.Panel>
                  {results.length ? (
                    results.map((result, index) => <RoleOutput key={index} result={result} />)
                  ) : (
                    <Text size="xs" c="dimmed">
                      No recorded output for this stage.
                    </Text>
                  )}
                </Accordion.Panel>
              </Accordion.Item>
              <Accordion.Item value="definitions">
                <Accordion.Control>Executable definitions</Accordion.Control>
                <Accordion.Panel>
                  {definitionLoading && (
                    <Group gap="xs">
                      <Loader size="xs" />
                      <Text size="xs">Reading stage files…</Text>
                    </Group>
                  )}
                  {definitionIssue && (
                    <Alert color="red">
                      <Text size="xs">{definitionIssue}</Text>
                      <Button size="compact-xs" variant="light" mt="xs" onClick={retryDefinition}>
                        Retry definitions
                      </Button>
                    </Alert>
                  )}
                  {definition?.definitions.map((role) => (
                    <div key={role.role}>
                      <Text ff="monospace" size="xs" fw={600} mt="sm">
                        {role.role}
                      </Text>
                      <Text size="xs" c="dimmed" className={styles.longText}>
                        {role.path}
                      </Text>
                      {role.issue ? (
                        <Text size="xs" c="red.3">
                          {role.issue}
                        </Text>
                      ) : (
                        <pre className={styles.code}>{role.contents}</pre>
                      )}
                    </div>
                  ))}
                </Accordion.Panel>
              </Accordion.Item>
            </Accordion>
          </>
        )}
      </div>
    </Stack>
  )
}
type InspectorProps = {
  open: boolean
  stage: StageView | null
  definition?: DefinitionView
  definitionIssue: string | null
  definitionLoading: boolean
  checkpoint: CheckpointView | null
  observation: MovementObservation | null
  onCollapse: () => void
  retryDefinition: () => void
}
