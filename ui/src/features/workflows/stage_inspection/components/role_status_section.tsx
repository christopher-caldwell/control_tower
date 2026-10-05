import { Accordion } from '@mantine/core'
import type { FC } from 'react'

import type { MovementObservation, RoleObservation, StageView } from '@/api/types'
import { RolePreview } from '@/features/workflows/stage_inspection/components/role_output'

type RoleStatusSectionProps = {
  kind: 'mutation' | 'verification'
  stage: StageView
  results: RoleObservation[]
  failure: MovementObservation['failure']
  pendingDirection: 'up' | 'down' | undefined
}

export const RoleStatusSection: FC<RoleStatusSectionProps> = ({ kind, stage, results, failure, pendingDirection }) => {
  const sectionLabel = kind === 'mutation' ? 'Mutation' : 'Verification'
  const roles = kind === 'mutation' ? ['up', 'down'] : ['verify-up', 'verify-down']
  return (
    <Accordion.Item value={kind}>
      <Accordion.Control>{sectionLabel}</Accordion.Control>
      <Accordion.Panel>
        {roles.map((role) => (
          <RolePreview
            key={role}
            role={role}
            stage={stage}
            results={results}
            failure={failure}
            pendingDirection={pendingDirection}
          />
        ))}
      </Accordion.Panel>
    </Accordion.Item>
  )
}
