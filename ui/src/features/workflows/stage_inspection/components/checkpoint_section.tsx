import { Accordion, Text } from '@mantine/core'
import type { FC } from 'react'

import type { CheckpointView, StageView } from '@/api/types'

type CheckpointSectionProps = { stage: StageView; checkpoint: CheckpointView | null }

export const CheckpointSection: FC<CheckpointSectionProps> = ({ stage, checkpoint }) => {
  let status = 'Not applied'
  if (stage.state === 'accepted') status = 'Applied'
  if (stage.state === 'pending') status = 'Pending ' + (checkpoint?.pending_transition?.direction ?? '')
  const positionText = stage.is_accepted_checkpoint ? 'Last confirmed accepted position' : 'Current workflow state'
  return (
    <Accordion.Item value="state">
      <Accordion.Control>Checkpoint state</Accordion.Control>
      <Accordion.Panel>
        <Text size="sm">{status}</Text>
        <Text size="xs" c="dimmed">
          {positionText}
        </Text>
      </Accordion.Panel>
    </Accordion.Item>
  )
}
