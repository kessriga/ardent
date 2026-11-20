# Design

## Context

The spec `ardent-layout` already gives the content of `.ardent/` after `ardent init` ("Initial content") and requires
that Ardent writes each file in `.ardent/` through `.ardent/cache/tmp/` ("Atomic writes"). The design of TASK-5 puts
`ardent init` in `ardent-cli` and the atomic write in `ardent-core`. Both crates hold only stubs today.

## Goals / Non-Goals

**Goals:**

- Meet "Initial content" and "Atomic writes" together: `ardent init` writes through `cache/tmp/`, and `.ardent/` still
  holds only `.gitignore` and `config.yaml` afterwards.
- Leave no half-built `.ardent/` after a failed `ardent init`, so that a second run can succeed.

**Non-Goals:**

- Nested projects. `cargo init` and `uv init` allow them because a nested project becomes a workspace member; Ardent has
  no workspaces, so a nested project would only split the views of the same files. `ardent init` refuses to run inside a
  project. It does not scan the directories below the working directory for an existing project; TASK-7 decides what the
  sync does with a nested `.ardent/`.
- A typed model of `config.yaml`. TASK-7 adds it when it parses the file.

## Decisions

### Crates

- `ardent-core` gains the module `layout`: the name `.ardent`, the paths of `cache/` and `cache/tmp/` in it, and
  `write_atomic`, which writes a file into `.ardent/` through a temporary file in `cache/tmp/`. TASK-7 and later tasks
  write structure and view files with the same function. It has no terminal or plugin-runtime code.
- `ardent-cli` gains the module `project_root`, which finds the nearest directory that holds a `.ardent/` directory, as
  the TASK-5 design places the root search in `ardent-cli`. It also gains the module `init`, which builds `.ardent/`,
  and the `init` subcommand in `main.rs`, which prints the result.

### Steps of `ardent init`

1. Search the parent directories of the working directory for a `.ardent/` directory, nearest first, and fail with the
   path of the first one found. The search starts at the parent, so step 2 alone handles a `.ardent` in the working
   directory, also when it is a file.
2. Create `.ardent/` with a call that fails when the path exists (`create_dir`, not `create_dir_all`). This one call is
   the existence check and the creation, so two `ardent init` runs at the same time cannot both succeed. A file named
   `.ardent` fails the same way.
3. Write `.gitignore`, then `config.yaml`, with `write_atomic`. The content is the literal text from "Initial content".
   A literal is the simplest way to match the spec byte for byte; TASK-7 adds the typed model for reading.
4. Remove the empty `cache/tmp/` and `cache/`, so that `.ardent/` holds only the two files. "Directories on demand"
   creates `cache/` again at the first sync.
5. If step 3 or 4 fails, remove `.ardent/` and report the first error. Step 2 created the directory, so `ardent init`
   removes only its own work.

Alternative: write the two files directly, without `cache/tmp/`. It is shorter, but a crash during the write of
`config.yaml` can leave a short file that still parses as a valid configuration with fewer formats. The atomic write
leaves the file either complete or absent.

### Exit status

`ardent init` exits with status 1 when `.ardent` exists or a parent directory is a project root. `cargo init` and
`uv init` also fail when the working directory is already a project. An agent that runs it sees that the command did
nothing. `git init` exits with 0 in this case, but it also updates the repository, and `ardent init` changes nothing.

### Atomic write

`write_atomic` creates the temporary file with `tempfile::NamedTempFile::new_in(cache/tmp/)`, writes the content, calls
`sync_all`, and renames the file into place with `persist`. `tempfile` gives unique names and removes the temporary file
when a step fails. `sync_all` keeps a view file complete after a power loss as well as after a crash.

### Command-line parsing

`clap` with the derive API parses the command line. It gives `--help`, `--version`, and errors for unknown commands and
arguments, and every later command uses it too.

## Risks / Trade-offs

- [A crash between step 2 and step 5 leaves a partial `.ardent/`, and the next `ardent init` reports that `.ardent`
  exists] → The message names the path, so the person can delete it and run `ardent init` again. A full fix needs a
  staging directory outside `.ardent/`, which "Init writes only inside .ardent/" forbids.
- [`ardent init` cannot test the clean-up after a failed write, because the test cannot make a write fail after
  `create_dir` succeeds] → The clean-up is one call; a review covers it.
