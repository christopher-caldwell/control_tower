export class ApiFailure extends Error {
  constructor(
    public code: string,
    message: string,
    public status: number,
  ) {
    super(message)
  }
}

export const apiRequest = async (path: string, init?: RequestInit): Promise<Response> => {
  const response = await fetch(path, init)
  if (response.ok) return response

  const payload = (await response.json().catch(() => null)) as {
    error?: { code?: string; message?: string }
  } | null
  throw new ApiFailure(
    payload?.error?.code ?? 'request_failed',
    payload?.error?.message ?? 'Request failed (' + response.status + ')',
    response.status,
  )
}

export const api = async <T>(path: string, signal?: AbortSignal): Promise<T> => {
  const response = await apiRequest(path, { signal })
  return response.status === 204 ? (undefined as T) : ((await response.json()) as T)
}
