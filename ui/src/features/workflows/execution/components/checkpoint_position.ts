import type { CheckpointView } from '@/api/types'

export const checkpointPosition = (checkpoint: CheckpointView | null): string => {
  const stage = checkpoint?.accepted_stage
  if (stage) return 'Stage ' + stage.number + ' · ' + stage.name
  return 'Baseline · no accepted stages'
}
