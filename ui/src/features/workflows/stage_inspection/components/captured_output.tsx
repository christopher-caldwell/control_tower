import { Accordion, Text } from '@mantine/core'
import type { FC } from 'react'

import type { RoleObservation } from '@/api/types'
import { RoleOutput } from '@/features/workflows/stage_inspection/components/role_output'

type CapturedOutputProps = { results: RoleObservation[] }

export const CapturedOutput: FC<CapturedOutputProps> = ({ results }) => {
  const hasResults = results.length > 0
  return (
    <Accordion.Item value="output">
      <Accordion.Control>Captured output</Accordion.Control>
      <Accordion.Panel>
        {hasResults ? (
          results.map((result, index) => <RoleOutput key={index} result={result} />)
        ) : (
          <Text size="xs" c="dimmed">
            No recorded output for this stage.
          </Text>
        )}
      </Accordion.Panel>
    </Accordion.Item>
  )
}
