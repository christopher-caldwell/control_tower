import type { FC } from 'react'

import { Workbench } from '@/app/components/workbench/workbench'
import { AppProviders } from '@/app/providers'

export const App: FC = () => (
  <AppProviders>
    <Workbench />
  </AppProviders>
)
