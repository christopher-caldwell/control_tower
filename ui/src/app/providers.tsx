import '@mantine/core/styles.css'

import { createTheme, MantineProvider } from '@mantine/core'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { FC, PropsWithChildren } from 'react'
import { useState } from 'react'

const theme = createTheme({
  primaryColor: 'violet',
  defaultRadius: 'sm',
  fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
  fontFamilyMonospace: "'SFMono-Regular', Consolas, monospace",
  fontSizes: { xs: '12px', sm: '14px', md: '14px', lg: '16px', xl: '20px' },
  headings: {
    sizes: { h1: { fontSize: '26px' }, h2: { fontSize: '18px' }, h3: { fontSize: '18px' }, h4: { fontSize: '14px' } },
  },
  colors: {
    dark: [
      '#e6e5ec',
      '#bab8c5',
      '#92909d',
      '#686674',
      '#42404b',
      '#35333f',
      '#29282f',
      '#232529',
      '#191d21',
      '#14171b',
    ],
  },
  other: { chrome: '#302268', navigation: '#1d1836', selection: '#70d18e' },
})
export const AppProviders: FC<AppProvidersProps> = ({ children }) => {
  const [client] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: { retry: false, refetchOnWindowFocus: false, refetchOnReconnect: false },
          mutations: { retry: false },
        },
      }),
  )
  return (
    <QueryClientProvider client={client}>
      <MantineProvider
        theme={theme}
        forceColorScheme="dark"
        cssVariablesResolver={(resolvedTheme) => ({
          variables: {
            '--mantine-other-chrome': resolvedTheme.other.chrome,
            '--mantine-other-navigation': resolvedTheme.other.navigation,
            '--mantine-other-selection': resolvedTheme.other.selection,
          },
          light: {},
          dark: {},
        })}
      >
        {children}
      </MantineProvider>
    </QueryClientProvider>
  )
}

type AppProvidersProps = PropsWithChildren
