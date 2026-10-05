import { afterEach, describe, expect, it, vi } from 'vitest'

import { api } from '@/api/client'

afterEach(() => vi.unstubAllGlobals())

describe('shared API client', () => {
  it('returns the ordinary inventory data', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(Response.json({ name: 'demo', workflows: [{ id: 'scratch', name: 'scratch' }] })),
    )
    await expect(api('/api/workspace')).resolves.toEqual({
      name: 'demo',
      workflows: [{ id: 'scratch', name: 'scratch' }],
    })
  })
})
