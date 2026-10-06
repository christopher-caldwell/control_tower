import { type ChildProcess, spawn } from 'node:child_process'
import { access, cp, mkdir, realpath, writeFile } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import type { Plugin } from 'vite'

const repo = resolve(dirname(fileURLToPath(import.meta.url)), '../..')

const findUiStartupUrl = (output: string): string | undefined => {
  const prefix = 'Control Tower UI: '
  const prefixIndex = output.indexOf(prefix)
  if (prefixIndex < 0) return undefined

  const escape = String.fromCharCode(27)
  const hyperlinkOpen = `${escape}]8;;`
  const hyperlinkClose = `${escape}\\`
  let urlStart = prefixIndex + prefix.length
  let hyperlinkTarget: string | undefined
  if (output.startsWith(hyperlinkOpen, urlStart)) {
    const targetStart = urlStart + hyperlinkOpen.length
    const closeIndex = output.indexOf(hyperlinkClose, targetStart)
    if (closeIndex < 0) return undefined
    hyperlinkTarget = output.slice(targetStart, closeIndex)
    urlStart = closeIndex + hyperlinkClose.length
  }

  const url = /^http:\/\/127\.0\.0\.1:\d+/.exec(output.slice(urlStart))?.[0]
  return url && (hyperlinkTarget === undefined || hyperlinkTarget === url) ? url : undefined
}

/** Own the real API and sample workspace for a Vite development session. */
export const workspaceBackend = (): Plugin => {
  const children = new Set<ChildProcess>()
  let stopping = false
  const stop = () => {
    stopping = true
    for (const child of children) child.kill('SIGTERM')
  }
  const interrupt = () => {
    stop()
    process.exit(130)
  }
  const terminate = () => {
    stop()
    process.exit(143)
  }
  const launch = (command: string, args: string[], cwd: string, capture = false) => {
    const child = spawn(command, args, { cwd, stdio: ['ignore', capture ? 'pipe' : 'inherit', 'inherit'] })
    children.add(child)
    child.once('exit', () => children.delete(child))
    return child
  }
  const run = (command: string, args: string[], cwd: string) =>
    new Promise<void>((resolve, reject) => {
      const child = launch(command, args, cwd)
      child.once('error', reject)
      child.once('exit', (code, signal) => {
        if (code === 0) resolve()
        else reject(new Error(`${command} ${args.join(' ')} failed (${signal ?? code}).`))
      })
    })
  const cleanup = () => {
    stop()
    process.off('exit', stop)
    process.off('SIGINT', interrupt)
    process.off('SIGTERM', terminate)
  }

  return {
    name: 'control-tower-workspace-backend',
    apply: (_, environment) => environment.command === 'serve' && !['test', 'browser-test'].includes(environment.mode),
    async config() {
      process.once('exit', stop)
      process.once('SIGINT', interrupt)
      process.once('SIGTERM', terminate)
      try {
        // Rust embeds the packaged frontend even when Vite serves the live source.
        try {
          await access(join(repo, 'ui/dist/index.html'))
        } catch {
          console.info('\n[control-tower] Preparing frontend assets for the first Rust build…')
          await run('pnpm', ['build'], join(repo, 'ui'))
        }
        console.info('\n[control-tower] Building the development API…')
        // Keep dev tooling independent of a caller's CARGO_TARGET_DIR.
        const target = join(repo, 'target')
        await run('cargo', ['build', '--locked', '-p', 'control-tower-cli', '--target-dir', target], repo)
        const binary = join(target, 'debug/control-tower')
        const configuredWorkspace = process.env.CONTROL_TOWER_DEV_WORKSPACE
        let workspace: string
        if (configuredWorkspace !== undefined) {
          if (!configuredWorkspace.trim()) throw new Error('CONTROL_TOWER_DEV_WORKSPACE must be a workspace path.')
          workspace = await realpath(resolve(configuredWorkspace))
          await realpath(join(workspace, 'workflows'))
        } else {
          workspace = join(repo, 'ui/.dev/workspace')
          const workflow = join(workspace, 'workflows/uuid-file')
          await mkdir(join(workspace, 'workflows'), { recursive: true })
          await writeFile(
            join(workspace, 'control-tower.toml'),
            '[workspace]\nlabel = "Control Tower UI development"\n',
          )
          // Refresh authored roles while preserving the development checkpoint and effects.
          await cp(join(repo, 'examples/simple/workflows/uuid-file'), workflow, {
            recursive: true,
            filter: (source) => !['.control_tower', 'data'].includes(source.split('/').at(-1) ?? ''),
          })
          for (const operation of ['bootstrap-local', 'migrate-local', 'verify-local']) {
            await run(binary, ['db', operation, '--workflow', workflow], workspace)
          }
        }
        console.info(`[control-tower] Workspace: ${workspace}`)
        const backend = launch(binary, ['ui'], workspace, true)
        const targetUrl = await new Promise<string>((resolve, reject) => {
          let output = ''
          const timer = setTimeout(() => {
            backend.kill('SIGTERM')
            reject(new Error('Control Tower API did not become ready within 15 seconds.'))
          }, 15_000)
          const fail = (error: Error) => {
            clearTimeout(timer)
            reject(error)
          }
          backend.once('error', fail)
          backend.once('exit', (code, signal) => fail(new Error(`Control Tower API exited (${signal ?? code}).`)))
          backend.stdout!.on('data', (chunk: Buffer) => {
            output += chunk.toString()
            const url = findUiStartupUrl(output)
            if (url) {
              clearTimeout(timer)
              resolve(url)
            }
          })
        })
        backend.once('exit', (code, signal) => {
          if (!stopping)
            console.error(`[control-tower] API stopped unexpectedly (${signal ?? code}); restart pnpm dev.`)
        })
        console.info(`[control-tower] API: ${targetUrl}; open the Vite URL below.\n`)
        return { server: { proxy: { '/api': { target: targetUrl } }, watch: { ignored: ['**/.dev/**'] } } }
      } catch (error) {
        cleanup()
        throw error
      }
    },
    closeBundle: cleanup,
  }
}
