import { useQuery } from '@tanstack/react-query'

import type { CheckpointView, MovementObservation, StageView } from '@/api/types'
import { definitionOptions } from '@/features/workflows/stage_inspection/api/options'

export type StageInspectionInput = {
  workflowId: string | null
  stage: StageView | null
  checkpoint: CheckpointView | null
  observation: MovementObservation | null
}
export type StageInspectionModel = ReturnType<typeof useStageInspection>

export const useStageInspection = (input: StageInspectionInput) => {
  const definitionQuery = useQuery(definitionOptions(input.workflowId, input.stage?.number ?? null))
  const selectedStageNumber = input.stage?.number ?? null
  const hasSelectedStage = selectedStageNumber !== null
  const hasRetainedResults = !hasSelectedStage && (input.observation?.role_results.length ?? 0) > 0
  const retainedObservation = hasRetainedResults ? input.observation : null
  const stageResults = hasSelectedStage
    ? (input.observation?.role_results.filter((result) => result.stage.number === selectedStageNumber) ?? [])
    : []
  const observedFailure = input.observation?.failure
  const isFailureForSelectedStage = Boolean(hasSelectedStage && observedFailure?.stage?.number === selectedStageNumber)
  const hasResultsForSelectedStage = hasSelectedStage && stageResults.length > 0
  const hasStageFailure = Boolean(observedFailure && (isFailureForSelectedStage || hasResultsForSelectedStage))

  return {
    stage: input.stage,
    checkpoint: input.checkpoint,
    observation: input.observation,
    retainedObservation,
    results: stageResults,
    failure: hasStageFailure ? (observedFailure ?? null) : null,
    definition: definitionQuery.data,
    isDefinitionLoading: hasSelectedStage && definitionQuery.isPending,
    definitionIssue: definitionQuery.error instanceof Error ? definitionQuery.error.message : null,
    retryDefinition: () => void definitionQuery.refetch(),
  }
}
