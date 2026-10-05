import type { FC } from 'react'

import { AppProviders } from '@/app/providers'
import { Workbench } from '@/features/workflows/workbench'
export const App: FC = () => (
  <AppProviders>
    <Workbench />
  </AppProviders>
)
