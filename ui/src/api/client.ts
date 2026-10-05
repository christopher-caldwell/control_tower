import type { CheckpointState, MovementChoice } from '@/api/types'
export class ApiFailure extends Error {
  constructor(
    public code: string,
    message: string,
    public status: number,
  ) {
    super(message)
  }
}
export const api = async <T>(path: string, signal?: AbortSignal): Promise<T> => {
  const response = await fetch(path, { signal })
  if (!response.ok) {
    const payload = (await response.json().catch(() => null)) as { error?: { code?: string; message?: string } } | null
    throw new ApiFailure(
      payload?.error?.code ?? 'request_failed',
      payload?.error?.message ?? `Request failed (${response.status})`,
      response.status,
    )
  }
  return response.status === 204 ? (undefined as T) : ((await response.json()) as T)
}
export const submitMovement = async (
  path: string,
  movement: MovementChoice,
  expected_checkpoint: CheckpointState,
): Promise<void> => {
  const response = await fetch(path, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ ...movement, expected_checkpoint }),
  })
  if (!response.ok) {
    const payload = (await response.json().catch(() => null)) as { error?: { code?: string; message?: string } } | null
    throw new ApiFailure(
      payload?.error?.code ?? 'movement_failed',
      payload?.error?.message ?? `Movement request failed (${response.status})`,
      response.status,
    )
  }
}
