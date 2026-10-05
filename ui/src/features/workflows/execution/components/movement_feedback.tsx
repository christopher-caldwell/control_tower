import { Alert } from '@mantine/core'
import type { FC } from 'react'

type MovementFeedbackProps = { issue: string | null }

export const MovementFeedback: FC<MovementFeedbackProps> = ({ issue }) =>
  issue ? (
    <Alert color="red" role="status">
      {issue}
    </Alert>
  ) : null
