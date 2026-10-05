import { Text } from '@mantine/core'
import type { FC } from 'react'

import styles from '@/features/workflows/stage_inspection/components/stage_inspector.module.css'

type OutputStreamProps = { name: 'stdout' | 'stderr'; contents: string | null }

export const OutputStream: FC<OutputStreamProps> = ({ name, contents }) => (
  <div>
    <Text size="xs" c={name === 'stderr' ? 'orange.3' : 'teal.3'} mt="sm">
      {name}
    </Text>
    {contents ? (
      <pre className={styles.code} aria-label={name}>
        {contents}
      </pre>
    ) : (
      <Text size="xs" c="dimmed">
        Empty stream
      </Text>
    )}
  </div>
)
