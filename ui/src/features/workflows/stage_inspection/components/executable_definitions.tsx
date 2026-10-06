import { Accordion, Alert, Button, Group, Loader, Text } from '@mantine/core'
import type { FC } from 'react'

import type { DefinitionView } from '@/api/types'
import styles from '@/features/workflows/stage_inspection/components/stage_inspector.module.css'

type ExecutableDefinitionsProps = {
  definition: DefinitionView | undefined
  issue: string | null
  isLoading: boolean
  onRetry: () => void
}

export const ExecutableDefinitions: FC<ExecutableDefinitionsProps> = ({ definition, issue, isLoading, onRetry }) => (
  <Accordion.Item value="definitions">
    <Accordion.Control>Stage Actions</Accordion.Control>
    <Accordion.Panel>
      {isLoading ? (
        <Group gap="xs">
          <Loader size="xs" />
          <Text size="xs">Reading stage files…</Text>
        </Group>
      ) : null}
      {issue ? (
        <Alert color="red">
          <Text size="xs">{issue}</Text>
          <Button size="compact-xs" variant="light" mt="xs" onClick={onRetry}>
            Retry definitions
          </Button>
        </Alert>
      ) : null}
      {definition?.definitions.map((item) => (
        <div key={item.role}>
          <Text ff="monospace" size="xs" fw={600} mt="sm">
            {item.role}
          </Text>
          <Text size="xs" c="dimmed" className={styles.longText}>
            {item.path}
          </Text>
          {item.issue ? (
            <Text size="xs" c="red.3">
              {item.issue}
            </Text>
          ) : (
            <pre className={styles.code}>{item.contents}</pre>
          )}
        </div>
      ))}
    </Accordion.Panel>
  </Accordion.Item>
)
