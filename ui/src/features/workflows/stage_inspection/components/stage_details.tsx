import { Accordion, Alert, Badge, Stack, Text, Title } from '@mantine/core'
import type { FC } from 'react'

import { CapturedOutput } from '@/features/workflows/stage_inspection/components/captured_output'
import { CheckpointSection } from '@/features/workflows/stage_inspection/components/checkpoint_section'
import { ExecutableDefinitions } from '@/features/workflows/stage_inspection/components/executable_definitions'
import { RoleStatusSection } from '@/features/workflows/stage_inspection/components/role_status_section'
import styles from '@/features/workflows/stage_inspection/components/stage_inspector.module.css'
import type { StageInspectionModel } from '@/features/workflows/stage_inspection/hooks/use_stage_inspection'

type StageDetailsProps = { model: StageInspectionModel }

export const StageDetails: FC<StageDetailsProps> = ({ model }) => {
  const stage = model.stage
  if (!stage) return null
  const stateColors = { accepted: 'green', pending: 'yellow', future: 'gray' } as const
  const stateColor = stateColors[stage.state]
  let stateLabel = 'Future stage'
  if (stage.state === 'accepted') stateLabel = 'Applied'
  if (stage.state === 'pending') stateLabel = 'Pending'
  const pendingDirection = model.checkpoint?.pending_transition?.direction

  return (
    <>
      <Stack gap={0} p="md">
        <Text size="xs" c="dimmed">
          STAGE {stage.number}
        </Text>
        <Title order={3} mt={4} className={styles.longText}>
          {stage.name}
        </Title>
        <Badge w="fit-content" mt="sm" color={stateColor}>
          {stateLabel}
        </Badge>
      </Stack>
      {model.failure ? <FailureNotice model={model} /> : null}
      <Accordion multiple defaultValue={['state', 'mutation', 'verification', 'output', 'definitions']}>
        <CheckpointSection stage={stage} checkpoint={model.checkpoint} />
        <RoleStatusSection
          kind="mutation"
          stage={stage}
          results={model.results}
          failure={model.failure}
          pendingDirection={pendingDirection}
        />
        <RoleStatusSection
          kind="verification"
          stage={stage}
          results={model.results}
          failure={model.failure}
          pendingDirection={pendingDirection}
        />
        <CapturedOutput results={model.results} />
        <ExecutableDefinitions
          definition={model.definition}
          issue={model.definitionIssue}
          isLoading={model.isDefinitionLoading}
          onRetry={model.retryDefinition}
        />
      </Accordion>
    </>
  )
}

const FailureNotice: FC<StageDetailsProps> = ({ model }) => {
  const failure = model.failure
  if (!failure) return null
  const title = failure.kind === 'checkpoint_save_failed' ? 'Checkpoint was not confirmed' : 'Observed failure'
  return (
    <Alert color="red" m="md" title={title}>
      {failure.message}
    </Alert>
  )
}
