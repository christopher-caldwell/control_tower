# Control Tower workbench UI

This React client follows the installed `playbook react_app` guidance. The Rust CLI embeds the generated `dist/` assets at build time and serves them on the same loopback origin as its API.

## Project declarations

- Desktop only: three simultaneous panes, a fixed 382px inspector, a 252px workflow rail, manual session-only collapse, and independent scrolling. Narrow windows retain the horizontal workbench.
- Workflow/stage selection is transient inspection state. The Application controls legal movement and recovery choices.
- Keyboard-operable controls, visible focus, meaningful accessible names, text status indicators, and inert output rendering are required. No formal WCAG conformance claim is made.
- Existing HTTP/SSE live snapshots remain the only workflow status channel. No polling, automatic Query retries, or movement resubmission.
- No authentication, forms, tables, URL routing, notification service, or persistent attempt history is active.
- No OpenAPI schema exists. `src/api/types.ts` mirrors the Rust web DTOs directly; there is no feature transport remap. Revisit generation when an authoritative schema is introduced.

## Ownership

`src/app/` owns Mantine theme and Query providers. `src/api/` owns transport types, the HTTP client, and error normalization. The app-owned workbench composes three public capabilities. Their entry points export each headless hook, standard UI, model/input type, and UI prop type:

| Feature                      | Headless facade                                                      | Standard UI         |
| ---------------------------- | -------------------------------------------------------------------- | ------------------- |
| `workspace/workflows`        | `useWorkspaceWorkflows()`                                            | `WorkflowRail`      |
| `workflows/execution`        | `useWorkflowExecution({ workflowId })`                               | `WorkflowExecution` |
| `workflows/stage_inspection` | `useStageInspection({ workflowId, stage, checkpoint, observation })` | `StageInspector`    |

The app calls each facade once, passes its model to the standard UI, connects workflow selection to execution, and passes the selected stage context to inspection. Each feature root contains only `index.ts`; hooks, API, and components live under responsibility folders. Query keys, cache handling, transport calls, and request coordination stay private. This follows the accepted facade and presentation decision UI-DEC-0001 in the installed React Playbook.

Execution's private `model/` owns movement action construction used by its headless facade. Checkpoint display formatting remains in `components/`; headless behavior does not depend on presentation modules.

## Commands

### Develop against a real workspace

From the repository root:

```sh
pnpm --dir ui install --frozen-lockfile
just dev-ui
```

Or run `pnpm dev` from `ui/`. Open the **Vite URL** printed in the terminal.
The dev server builds and starts the checkout's Rust API, prepares a shell-only
`uuid-file` sample under `ui/.dev/workspace`, and proxies API requests and SSE to
that API. React changes hot reload. Ctrl-C stops both servers. Rust changes require
restarting the dev command. Rust/Cargo and a Unix shell are required alongside pnpm;
no separately installed `control-tower` binary or sample dependencies are needed.

The sample's checkpoint and fixture data survive restarts. Use the UI's backward
movement to return to stage 0. To recreate the sample completely, stop the dev
server, remove `ui/.dev/workspace`, and start it again. Generated sample files are
ignored by Git and frontend watchers. Authored sample files refresh on startup.

To use an existing workspace instead, provide an absolute path:

```sh
CONTROL_TOWER_DEV_WORKSPACE=/absolute/path/to/workspace pnpm --dir ui dev
```

That directory must contain `workflows/`. Its workflows must already have their
databases and dependencies prepared; the dev server only prepares its own sample.
Movement controls execute the selected workspace's real roles. Restart to change
workspaces or refresh the workflow inventory.

Browser tests use `--mode browser-test` and their mocked API; unit tests and
production builds do not start the development backend.

### Verify and build

Use the pinned pnpm version in `package.json`:

```sh
pnpm install --frozen-lockfile
pnpm format:check
pnpm lint
pnpm typecheck
pnpm test
pnpm exec playwright install chromium
pnpm test:browser
pnpm build
```

Use `pnpm format` to format source. Rebuild the ignored `dist/` assets after UI changes, then build the Rust workspace with `cargo build --locked --workspace` from the repository root. A fresh checkout also needs the UI build before its first Rust build; generated assets are not committed.

TypeScript 7 remains the build/typecheck compiler. ESLint currently requires the TypeScript 6 programmatic API, so `typescript` aliases `@typescript/typescript6`, while `@typescript/native` aliases the pinned TypeScript 7 package. This follows [Microsoft's side-by-side setup](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/#running-side-by-side-with-typescript-6.0).
