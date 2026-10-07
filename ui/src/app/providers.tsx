import '@mantine/core/styles.css'
import '@mantine/code-highlight/styles.css'
import 'highlight.js/styles/github-dark.css'

import { CodeHighlightAdapterProvider, createHighlightJsAdapter } from '@mantine/code-highlight'
import { createTheme, MantineProvider } from '@mantine/core'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import hljs from 'highlight.js/lib/core'
import bash from 'highlight.js/lib/languages/bash'
import javascript from 'highlight.js/lib/languages/javascript'
import python from 'highlight.js/lib/languages/python'
import typescript from 'highlight.js/lib/languages/typescript'
import type { FC, PropsWithChildren } from 'react'
import { useState } from 'react'

hljs.registerLanguage('bash', bash)
hljs.registerLanguage('javascript', javascript)
hljs.registerLanguage('python', python)
hljs.registerLanguage('typescript', typescript)
const codeAdapter = createHighlightJsAdapter(hljs)

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
        <CodeHighlightAdapterProvider adapter={codeAdapter}>{children}</CodeHighlightAdapterProvider>
      </MantineProvider>
    </QueryClientProvider>
  )
}

type AppProvidersProps = PropsWithChildren
