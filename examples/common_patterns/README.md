# Common patterns

This copyable Workspace shows how Stage Actions in separate Workflows can use
the same small, reusable libraries for ordinary author-owned plumbing. Both
Workflows capture the output of a local operation, save a generated value, and
consume that saved value in a later Stage Action. One uses Node and one uses
Python; both use only their language's standard library.

| Workflow | Prerequisites | Lesson |
| --- | --- | --- |
| [node](workflows/node/README.md) | Node.js | Reuse capture, JSON state, and check helpers from `lib/node/`. |
| [python](workflows/python/README.md) | Python 3 | Reuse the same pattern from `lib/python/`. |

`lib/` is an ordinary directory of author-owned project code. Control Tower
does not discover, load, or manage it; each Stage Action imports it explicitly.
Replace these helpers with any implementation that fits your project. They are
examples of reusable project infrastructure, not a supported Control Tower SDK
or a promise of SDK compatibility.

## Copy and run

From the Control Tower repository, copy the complete Workspace:

```sh
example="$(mktemp -d /tmp/control-tower-common-patterns.XXXXXX)/common_patterns"
cp -R examples/common_patterns "$example"
cd "$example"
```

Choose a Workflow and prepare it using the ordinary Control Tower commands:

```sh
workflow=workflows/node
control-tower db bootstrap-local --workflow "$workflow"
control-tower db migrate-local --workflow "$workflow"
control-tower db verify-local --workflow "$workflow"
control-tower validate --workflow "$workflow"
control-tower up --workflow "$workflow" --stage 2
cat "$workflow/data/state.json"
control-tower down --workflow "$workflow" --stage 0
```

Use `workflow=workflows/python` for the Python version. Each complete forward
and reset traversal uses the helper library rooted at the copied Workspace.
Stage Actions run as separate processes; the saved JSON file is an explicit,
author-owned handoff. Control Tower does not interpret its contents. Any
atomicity required across several operations belongs inside one Stage Action;
separate Stage Actions are not transactional.

From the repository root, run `./examples/test --family common_patterns` to
copy both Workflows and exercise their forward and reset paths.
