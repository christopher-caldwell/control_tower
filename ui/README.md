# Control Tower workbench UI

This React client follows the installed `playbook react_app` guidance. The Rust CLI serves the committed `dist/` assets on the same loopback origin as its API.

## Project declarations

- Desktop only: three simultaneous panes, a fixed 382px inspector, a 252px workflow rail, manual session-only collapse, and independent scrolling. Narrow windows retain the horizontal workbench.
- Workflow/stage selection is transient inspection state. The Application controls legal movement and recovery choices.
- Keyboard-operable controls, visible focus, meaningful accessible names, text status indicators, and inert output rendering are required. No formal WCAG conformance claim is made.
- Existing HTTP/SSE live snapshots remain the only workflow status channel. No polling, automatic Query retries, or movement resubmission.
- No authentication, forms, tables, URL routing, notification service, or persistent attempt history is active.
- No OpenAPI schema exists. `src/api/types.ts` mirrors the Rust web DTOs directly; there is no feature transport remap. Revisit generation when an authoritative schema is introduced.

## Ownership

`src/app/` owns Mantine theme and Query providers. `src/api/` owns transport types, the HTTP client, and error normalization. `src/features/workflows/workbench/` exposes its facade and standard UI from `index.ts`; API calls, keys, snapshot parsing, presentation helpers, and components stay private. The default UI consumes the same facade exposed to alternative consumers.

## Commands

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

Use `pnpm dev` for development and `pnpm format` to format source. Rebuild and commit `dist/` after UI changes, then build the Rust workspace. Browser tests retain screenshots under `../output/playwright/` for review.

TypeScript 7 remains the build/typecheck compiler. ESLint currently requires the TypeScript 6 programmatic API, so `typescript` aliases `@typescript/typescript6`, while `@typescript/native` aliases the pinned TypeScript 7 package. This follows [Microsoft's side-by-side setup](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/#running-side-by-side-with-typescript-6.0).
