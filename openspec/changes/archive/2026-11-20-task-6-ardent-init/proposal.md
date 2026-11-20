# Proposal

## Why

Every Ardent command except `ardent init` starts at the project root, and the project root is the directory that holds
`.ardent/`. Until `ardent init` exists, no project can exist, and TASK-7 to TASK-9 have nothing to run in. This change
implements Backlog TASK-6.

## What Changes

- Add the `ardent init` command. It creates `.ardent/` in the working directory with the two files that the spec
  `ardent-layout` gives under "Initial content": `.gitignore` and `config.yaml`.
- Specify what `ardent init` does when the working directory already contains `.ardent`: it fails, names the path, and
  changes no file.
- Specify that `ardent init` fails inside an existing project: when a parent directory of the working directory holds
  `.ardent/`, a nested project would track the same files as the outer project and give their nodes other node paths.
- Specify that `ardent init` writes nothing outside `.ardent/`.
- Add the first code that writes into `.ardent/`: an atomic write through `.ardent/cache/tmp/`, as the spec
  `ardent-layout` requires under "Atomic writes".

Out of scope:

- `ardent init` in a parent directory of an existing project. Only a scan of every directory below could find it; the
  sync in TASK-7 decides how to treat a nested `.ardent/`.
- Reading or validating `config.yaml`. TASK-7 parses it.
- The lock on `.ardent/cache/lock`. `ardent init` runs before any sync, so it takes no lock.
- Options for `ardent init`, such as a target directory or a choice of formats.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ardent-layout`: add requirements for `ardent init` in a directory that already contains `.ardent`, inside an existing
  project, and for writes outside `.ardent/`.

## Impact

- `ardent-core` gains a `layout` module with the name of the `.ardent/` directory and the atomic write.
- `ardent-cli` gains the project root search, the `init` command, and its first integration tests. TASK-7 uses the same
  search for every other command.
- New dependencies: `clap` in `ardent-cli` to parse commands, and `tempfile` in `ardent-core` for the temporary file of
  the atomic write. The tests also use `tempfile`.
