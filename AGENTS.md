# Ardent agent guide

Ardent is a Rust terminal outliner that derives a tree of nodes from structured source files (Markdown, Rust, HTML) and
attaches views to those nodes. Read `README.md` for the idea and the glossary. Use the glossary terms exactly: *node*,
*outline*, *node path*, *fingerprint*, *structure*, *sync*, *view*, *view entry*, *anchor*, *reference*, *backlink*,
*format plugin*, *view plugin*.

## Sources of truth

| Question                                        | Look in                                    |
| ----------------------------------------------- | ------------------------------------------ |
| What is the idea, and what do terms mean?       | `README.md`                                |
| What work exists, and what is its status?       | Backlog tasks and milestones (`backlog`)   |
| What did we decide, and why?                    | Backlog decisions (`backlog decision list`) |
| What did a spike or research find?              | Backlog docs (`backlog doc list`)          |
| How must the system behave?                     | `openspec/specs/`                          |
| What behavior change is in flight?              | `openspec/changes/`                        |

## The design is open

Milestone M0 holds spikes for the core design questions: anchors, sync, the `.ardent/` layout, and plugins. Before you
build anything that depends on one of these questions, find the accepted Backlog decision for it. If no decision exists,
stop and ask me. Do not settle a core design question inside implementation code.

## Workflow

1. Pick a Backlog task and follow the Backlog guides below.
2. For a **spike**, the output is a Backlog decision record, and a Backlog doc when the research is long. Keep throwaway
   prototype code out of `crates/`.
3. For a task that changes behavior, a CLI command, or an on-disk format, create an OpenSpec change named
   `task-<n>-<slug>` with `/opsx:propose`, and add it to the task with `--ref`. Implement with `/opsx:apply`. Archive
   the change with `/opsx:archive` in the same PR as the code.
4. For a refactor, a fix that keeps the specified behavior, or a chore, skip OpenSpec.

## Code layout

- `crates/ardent-core`: the document model. No terminal, file-watching, or plugin-runtime dependencies.
- `crates/ardent-cli`: the `ardent` binary. It depends on `ardent-core`.
- Dependency direction: `ardent-core ← ardent-cli`. Add crates (for example a TUI crate or a plugin host) when a task
  needs them, and keep the direction acyclic with `ardent-core` at the bottom.
- Workspace lints deny `unwrap`, `panic!`, and `unsafe`. Return typed errors from library crates.

## Verification gate

Run all of these before you report work as done:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
rumdl fmt --no-cache --enable MD013 <changed .md files>
```

There is no CI yet, so this gate is the contract.

## Autonomy

You may, without asking: add or change code and tests inside a task's scope, add a well-maintained dependency that the
task needs (state why in the commit), and update `README.md`, specs, and docs to match new behavior.

Ask me first before you: decide a core design question (see above), change the `.ardent/` format after it is specified,
create Backlog tasks or milestones, or rewrite Git history.

<!-- BACKLOG.MD GUIDELINES START -->
<!-- backlog.md-instructions-version: 1.53.0 -->
<CRITICAL_INSTRUCTION>

## Backlog.md Workflow

This project uses Backlog.md for task and project management.

**At the beginning of each conversation in this project, run `backlog instructions overview` before answering or taking
action. Re-read it only if you have not read it yet in the current conversation.**

Use the overview to decide whether to search, read, create, or update Backlog tasks.

Before task lifecycle actions, read the matching detailed guide:
- `backlog instructions task-creation` before creating or splitting tasks
- `backlog instructions task-execution` before planning, changing status or assignee, adding a plan or implementation
  notes, or implementing task work
- `backlog instructions task-finalization` before checking acceptance criteria, writing final summaries, or moving tasks
  to terminal statuses

Use `backlog <command> --help` before running unfamiliar commands. Help shows options, fields, and examples.

Do not edit Backlog task, draft, document, decision, or milestone markdown files directly. Use the `backlog` CLI so
metadata, relationships, and history stay consistent.

</CRITICAL_INSTRUCTION>
<!-- BACKLOG.MD GUIDELINES END -->
