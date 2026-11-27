# ardent-layout Specification

## Purpose

Defines where Ardent keeps each file in a project's `.ardent/` directory, what `ardent init` writes there, how the
project configuration selects plugins and tracked files, and where plugins live. Decision-4 defines the M1 layout.
Decision-8 defines the installed format package for TASK-12 and plugin selection for TASK-13.

## Requirements

### Requirement: Project root

The project root SHALL be the nearest directory, starting at the working directory and going up, that contains a
`.ardent/` directory. Every command other than `ardent init` SHALL use that project root. The search SHALL start at the
working directory with symbolic links resolved. When Ardent cannot check a directory for `.ardent/` for a reason other
than a missing entry, the command SHALL fail with a message that names the path.

#### Scenario: Command in a subdirectory

- **WHEN** a person runs an Ardent command in `docs/` of a project whose root contains `.ardent/`
- **THEN** the command uses the project root, not `docs/`

#### Scenario: No project

- **WHEN** a person runs an Ardent command other than `ardent init`, and no ancestor directory contains `.ardent/`
- **THEN** the command fails with a message that the directory is not inside an Ardent project, and writes no file

#### Scenario: Directory reached through a symbolic link

- **WHEN** `/p/docs` is a symbolic link to `/data/docs`, only `/p` contains `.ardent/`, and a person runs an Ardent
  command other than `ardent init` in `/p/docs`
- **THEN** the command fails with a message that the directory is not inside an Ardent project

#### Scenario: Directory that cannot be searched

- **WHEN** the operating system denies Ardent the permission to look for `.ardent` in a directory on the way up
- **THEN** the command fails with a message that names the path, and writes no file

### Requirement: Source paths

Every source path in an Ardent file SHALL be relative to the project root and SHALL use `/` between its parts, on every
platform.

#### Scenario: Path on Windows

- **WHEN** Ardent tracks the file `src\engine.rs` on Windows
- **THEN** every Ardent file writes its source path as `src/engine.rs`

### Requirement: User data and cache

`.ardent/` SHALL hold user data in `config.yaml`, `plugins/`, and `views/`, and derived data only in `cache/`. Ardent
SHALL write `.ardent/.gitignore` with the single line `/cache/`, so git ignores the cache and tracks the user data.

#### Scenario: Git status after a sync

- **WHEN** a sync has written structure files and the backlink index in a git repository
- **THEN** `git status` lists no file under `.ardent/cache/`

### Requirement: Initial content

`ardent init` SHALL write `.ardent/.gitignore` with the line `/cache/` and `.ardent/config.yaml` with schema 1, empty
views and empty excludes. It SHALL copy each selected release package into `.ardent/plugins/<name>/` and enable exactly
those names under `formats` with their default patterns.

#### Scenario: Files after init

- **WHEN** a person initializes a project with only Rust selected
- **THEN** `.ardent/plugins/rust/` contains the Rust release manifest, component and accompanying README
- **AND** `.ardent/plugins/` contains only `rust/`, and `formats` contains only `rust`
- **AND** the first sync scans `src/main.rs` and skips `README.md`

#### Scenario: No formats selected

- **WHEN** a person confirms an empty selection or runs `ardent init --plugins ''`
- **THEN** init creates a project with an empty `formats` mapping
- **AND** `.ardent/` contains only `.gitignore` and `config.yaml`

#### Scenario: A selected package cannot be installed

- **WHEN** init cannot read a file from a selected release package
- **THEN** init exits with a failure, names the file, and leaves the directory ready for another init

### Requirement: Init in an existing project

When the working directory already contains a file or directory named `.ardent`, `ardent init` SHALL fail with a message
that names that path, and SHALL change no file.

#### Scenario: Second init

- **WHEN** a person runs `ardent init` in a directory that already contains `.ardent/`
- **THEN** the command exits with a nonzero status and a message that `.ardent` already exists
- **AND** every file and directory in the working directory keeps its content

#### Scenario: File named .ardent

- **WHEN** a person runs `ardent init` in a directory that contains a file named `.ardent`
- **THEN** the command exits with a nonzero status and a message that `.ardent` already exists
- **AND** the file keeps its content

### Requirement: Init inside a project

When a parent directory of the working directory contains a `.ardent/` directory, `ardent init` SHALL fail with a
message that names the nearest such directory as the project root, and SHALL change no file.

#### Scenario: Init in a subdirectory

- **WHEN** a person runs `ardent init` in `docs/` of a project whose root contains `.ardent/`
- **THEN** the command exits with a nonzero status and a message that names the project root
- **AND** `docs/` contains no `.ardent`

### Requirement: Init writes only inside .ardent/

`ardent init` SHALL create, change, or delete no file or directory outside `.ardent/`.

#### Scenario: Directory with source files

- **WHEN** a person runs `ardent init` in a directory that contains `README.md` and `src/main.rs`
- **THEN** `.ardent/` is the only new entry in the working directory
- **AND** `README.md` and `src/main.rs` keep their content

### Requirement: Directories on demand

Ardent SHALL create `plugins/`, `views/`, `cache/`, and their subdirectories only when it first writes a file into them.

#### Scenario: First structure file

- **WHEN** the first sync after `ardent init` scans `src/engine.rs`
- **THEN** Ardent creates `.ardent/cache/files/src/engine.rs/` and writes `structure.yaml` into it

### Requirement: Structure file location

The structure file of a tracked source file SHALL be `.ardent/cache/files/<source path>/structure.yaml`.

#### Scenario: Rust file

- **WHEN** a sync scans `src/engine.rs`
- **THEN** its structure file is `.ardent/cache/files/src/engine.rs/structure.yaml`

### Requirement: View file location

The stored entries of one view for one source file SHALL be in `.ardent/views/<source path>/<view>.yaml`, where `<view>`
is the name of the view plugin.

#### Scenario: Comment on a function

- **WHEN** a person adds a comment with the `comments` view to `src/engine.rs#impl Engine/fn run`
- **THEN** Ardent writes the entry into `.ardent/views/src/engine.rs/comments.yaml`

### Requirement: Derived view file location

The output of a derived view for one source file SHALL be in `.ardent/cache/files/<source path>/<view>.yaml`.

#### Scenario: Complexity of a Rust file

- **WHEN** the `complexity` view computes entries for `src/engine.rs`
- **THEN** Ardent writes them into `.ardent/cache/files/src/engine.rs/complexity.yaml`

### Requirement: Backlink index location

The backlink index SHALL be `.ardent/cache/backlinks.yaml`. Ardent SHALL derive it from the view files only, and SHALL
rebuild it when it is missing.

#### Scenario: Deleted index

- **WHEN** a person deletes `.ardent/cache/backlinks.yaml` and then runs any Ardent command
- **THEN** the command rebuilds the index from the view files, with the same content as before the deletion

### Requirement: Cache can be deleted

Ardent SHALL rebuild every file in `.ardent/cache/` from the source files and the view files when the cache is missing.

#### Scenario: Deleted cache, unchanged sources

- **WHEN** a person deletes `.ardent/cache/`, changes no source file, and runs any Ardent command
- **THEN** the command rebuilds every structure file, derived view file, and the backlink index
- **AND** no view file changes

### Requirement: Empty files and directories

Ardent SHALL delete a view file that has no entries left, and SHALL remove each directory under `.ardent/views/` and
`.ardent/cache/files/` that becomes empty.

#### Scenario: Last entry deleted

- **WHEN** a person deletes the only entry in `.ardent/views/src/engine.rs/comments.yaml`, and `src/engine.rs` has no
  other view file
- **THEN** Ardent deletes `comments.yaml` and the directory `.ardent/views/src/engine.rs/`

### Requirement: Entries follow their anchors

A stored entry SHALL live in the view file, for its view, of the source file in its anchor path. When the source file in
an anchor path changes, Ardent SHALL move the entry into the view file of the new source file at the same sync.

#### Scenario: Node moved to another file

- **WHEN** a person moves `fn run` from `src/engine.rs` into `src/runner.rs` with the same body, and a sync runs
- **THEN** the anchor path of its comment becomes `src/runner.rs#fn run`
- **AND** the entry moves from `.ardent/views/src/engine.rs/comments.yaml` to
  `.ardent/views/src/runner.rs/comments.yaml`

#### Scenario: Source file renamed

- **WHEN** a person renames `src/engine.rs` to `src/core.rs`, and a sync runs
- **THEN** every entry anchored in `src/engine.rs` moves into the view files under `.ardent/views/src/core.rs/`

#### Scenario: Anchor path edited by hand

- **WHEN** an agent changes an anchor path in `.ardent/views/src/engine.rs/comments.yaml` to a node in `docs/spec.md`
- **THEN** the next sync moves the entry into `.ardent/views/docs/spec.md/comments.yaml`

#### Scenario: Orphan stays

- **WHEN** a sync cannot resolve the anchor of an entry, and the entry becomes an orphan
- **THEN** the entry stays in its view file, with its last known path

### Requirement: Plugin names

A plugin name SHALL start with a lowercase ASCII letter and contain only lowercase ASCII letters, digits, and hyphens.
The name `structure` SHALL be reserved. Ardent SHALL reject a plugin with an invalid or reserved name.

#### Scenario: Reserved name

- **WHEN** `.ardent/plugins/structure/plugin.yaml` exists, or `config.yaml` enables a view named `structure`
- **THEN** every Ardent command fails with a message that names the reserved plugin name

### Requirement: Format plugins in the configuration

`config.yaml` SHALL enable format plugins by name under `formats`. Only the listed format plugins SHALL run. `files` and
`prefixes` under a format SHALL replace the plugin's defaults; when a key is absent, the plugin's default SHALL apply.

#### Scenario: Pattern override

- **WHEN** `config.yaml` lists `markdown` with `files: ["docs/**/*.md"]`
- **THEN** Ardent tracks `docs/spec.md` and does not track `README.md`

#### Scenario: Format not listed

- **WHEN** `config.yaml` lists only `markdown` under `formats`
- **THEN** Ardent tracks no `.rs` file

#### Scenario: Prefix override

- **WHEN** `config.yaml` lists `markdown` with `prefixes: {spec: ["docs/**/*.md"]}`
- **THEN** the search prefix `spec` covers only Markdown files under `docs/`

### Requirement: View plugins in the configuration

`config.yaml` SHALL enable view plugins by name under `views`, as a mapping from the plugin name to its settings. Only
the listed view plugins SHALL run.

#### Scenario: View enabled

- **WHEN** `config.yaml` contains `views: {comments: {}}`
- **THEN** Ardent runs the `comments` view plugin and no other view plugin

### Requirement: Dashboard settings

In projects with the view host, Ardent SHALL read each enabled view plugin's dashboard settings from
`views.<plugin>.dashboards.<id>` in `.ardent/config.yaml`. Ardent SHALL pass only that dashboard's named fields to its
`render` call. Each value SHALL be text, a number, a boolean or a list of texts. The plugin SHALL validate its settings
and apply its defaults when fields are absent.

#### Scenario: Limit recent comments

- **WHEN** the project configures `views: {comments: {dashboards: {recent: {limit: 20}}}}` and opens recent comments
- **THEN** the host passes the numeric field `limit` with value `20` to the comments plugin's `recent` dashboard

#### Scenario: Dashboard uses its defaults

- **WHEN** the project enables the comments plugin with `views: {comments: {}}` and opens recent comments
- **THEN** the host passes an empty set of settings and the plugin applies its defaults

### Requirement: Patterns

Every pattern in `config.yaml` and in a plugin manifest SHALL use gitignore syntax, relative to the project root. A
pattern without `/` SHALL match at any depth.

#### Scenario: Pattern without a slash

- **WHEN** a format has the pattern `*.md`
- **THEN** it matches `README.md` and `docs/guide/intro.md`

### Requirement: Tracked files

Ardent SHALL track a file when the `files` patterns of exactly one enabled format match it, git's ignore rules do not
ignore it, no `exclude` pattern matches it, it is outside `.ardent/`, it is outside every nested project, and its path
is valid UTF-8. A nested project is a directory below the project root that contains its own `.ardent/` directory.

#### Scenario: Ignored by git

- **WHEN** `.gitignore` contains `target/` and `target/doc/notes.md` exists
- **THEN** Ardent does not track `target/doc/notes.md`

#### Scenario: Excluded

- **WHEN** `config.yaml` contains `exclude: ["vendor/"]`
- **THEN** Ardent does not track any file under `vendor/`

#### Scenario: Two formats match

- **WHEN** two enabled formats both match `notes.md`
- **THEN** the sync reports `notes.md` and the names of both formats
- **AND** Ardent does not track `notes.md`

#### Scenario: Path is not UTF-8

- **WHEN** a file matches a format pattern and its path is not valid UTF-8
- **THEN** the sync reports the file, and Ardent does not track it

#### Scenario: Nested project

- **WHEN** the project root holds `tools/.ardent/` and `tools/README.md`
- **THEN** the sync reports `tools/` as a nested project
- **AND** Ardent does not track `tools/README.md`

### Requirement: Format patterns match files

A `files` pattern SHALL match the path of the file itself. A file SHALL NOT match because a directory above it matches.

#### Scenario: Directory with the name of a format pattern

- **WHEN** the project holds `notes.md/image.png`
- **THEN** Ardent does not track `notes.md/image.png`

### Requirement: Ignore files above the project

Git's ignore rules SHALL include the `.gitignore` files above the project root up to the root of its git repository. No
`.gitignore` file outside a nested git repository SHALL apply inside it.

#### Scenario: Ignore file above the git repository

- **WHEN** `/work/.gitignore` contains `*.md`, `/work/repo/` is a git repository and the project root, and
  `/work/repo/README.md` exists
- **THEN** Ardent tracks `README.md`

### Requirement: Excluded directories

Ardent SHALL NOT read a directory that an `exclude` pattern matches.

#### Scenario: Unreadable directory under an excluded directory

- **WHEN** `config.yaml` contains `exclude: ["vendor/"]`, `vendor/secret/` cannot be read, and `vendor/lib/.ardent/`
  exists
- **THEN** a sync reports neither directory and exits with status 0

### Requirement: Plugin locations

Ardent SHALL load enabled format plugins only from `.ardent/plugins/<name>/`. The release SHALL supply Markdown and Rust
as separate packages. The CLI SHALL embed neither plugin.

#### Scenario: Pinned Markdown plugin

- **WHEN** `.ardent/plugins/markdown/plugin.yaml` and its component agree on version `0.0.9`, and a sync scans
  `README.md`
- **THEN** the stamp of the structure of `README.md` has `format: markdown` and `format_version: 0.0.9`

#### Scenario: New project without installed plugins

- **WHEN** a new project enables Markdown and Rust in its configuration but both project packages are missing
- **THEN** sync warns about both missing formats, scans zero files and exits with status 0

### Requirement: Plugin manifest

Each project plugin SHALL have a manifest `plugin.yaml` with `schema`, `name`, `version`, and `kind`. `name` SHALL equal
the name of its directory. `kind` SHALL be `format` or `view`. File names in the manifest SHALL be relative to the
plugin directory.

#### Scenario: Format manifest

- **WHEN** `.ardent/plugins/html/plugin.yaml` contains:

```yaml
schema: 1
name: html
version: 0.3.1
kind: format
component: format.wasm
files:
  - "*.html"
prefixes:
  page:
    - "*.html"
```

- **THEN** before using its defaults, Ardent SHALL verify the manifest metadata against `describe` on the supported
  `ardent:format/scanner@0.1.0` export
- **AND** Ardent MAY reuse a successful check for the same manifest bytes, component bytes and supported interface

#### Scenario: Changed manifest excludes all previously tracked files

- **WHEN** only the Rust manifest changes from `*.rs` to `*.txt`, the component still declares `*.rs`, and the project
  contains no text files
- **THEN** the sync SHALL report the mismatch, exit with status 1 and skip stale-structure deletion
- **AND** the existing Rust structures and stamps SHALL stay unchanged

#### Scenario: View manifest

- **WHEN** `.ardent/plugins/comments/plugin.yaml` contains:

```yaml
schema: 1
name: comments
version: 0.1.0
kind: view
component: comments.wasm
```

- **THEN** Ardent loads the view plugin from `.ardent/plugins/comments/comments.wasm`

#### Scenario: Name does not match the directory

- **WHEN** `.ardent/plugins/html/plugin.yaml` contains `name: xml`
- **THEN** every Ardent command fails with a message that names the manifest and both names

### Requirement: Lock

During a sync and the writes after it, Ardent SHALL hold an exclusive operating-system lock on `.ardent/cache/lock`. A
second Ardent process SHALL wait for the lock. The lock SHALL end when the process that holds it ends.

#### Scenario: TUI and CLI at the same time

- **WHEN** the TUI is in a sync and an agent runs an Ardent command
- **THEN** the command starts its own sync only after the TUI's sync has written its results

#### Scenario: Crash during a sync

- **WHEN** an Ardent process ends during a sync without releasing the lock
- **THEN** the next Ardent command gets the lock without waiting and without manual cleanup

### Requirement: Atomic writes

Ardent SHALL write each file in `.ardent/` into `.ardent/cache/tmp/` first and then rename it into place.

#### Scenario: Crash during a write

- **WHEN** an Ardent process ends while it writes a view file
- **THEN** the view file has either its old content or its new content
- **AND** any partly written file is under `.ardent/cache/tmp/`

### Requirement: Select release format plugins during init

In a terminal, `ardent init` SHALL offer the release format plugins and let the user select any subset in one prompt. An
explicit comma-separated `--plugins` value SHALL bypass the prompt. Init SHALL resolve the selection before creating the
project.

#### Scenario: Interactive selection

- **WHEN** a person runs `ardent init` in a terminal and selects Markdown and Rust
- **THEN** init installs and enables both packages

#### Scenario: Cancel the prompt

- **WHEN** a person cancels the picker before confirming a selection
- **THEN** init leaves the directory unchanged

#### Scenario: Scripted selection

- **WHEN** a script runs `ardent init --plugins markdown,rust` with standard input closed
- **THEN** init installs and enables both packages without prompting

#### Scenario: Missing flag outside a terminal

- **WHEN** a script runs `ardent init` without a terminal or a plugin selection
- **THEN** init exits with a failure, names `--plugins`, and creates no project

#### Scenario: Unknown name mixed with a valid name

- **WHEN** a person runs `ardent init --plugins markdown,html`
- **THEN** init exits with a failure, names `html`, lists `markdown` and `rust` as available, and creates no project

### Requirement: Update installed release format plugins

From any directory in a project, `ardent update` SHALL replace each differing, installed release format plugin with a
complete copy of its package from the running release. It SHALL compare package paths and file contents, preserve
project configuration and plugins with other identities, and skip release plugins the project has not installed. A
locally modified format plugin with the same directory and manifest name as a bundled format plugin SHALL count as that
bundled plugin.

#### Scenario: A component changes without a version change

- **WHEN** the bundled Markdown component differs from the installed component and both manifests give the same version
- **AND** the reader runs `ardent update` from a project subdirectory
- **THEN** the command names Markdown as updated and the installed package matches the complete bundled package

#### Scenario: The release changes only a supporting file

- **WHEN** the release changes a nested documentation file and removes another file from its Rust package
- **THEN** running `ardent update` installs the new file contents and removes the obsolete file

#### Scenario: Project installation choices survive an update

- **WHEN** Markdown is installed but disabled, Rust is absent, and a third-party plugin is installed
- **THEN** the command updates Markdown if it differs, leaves Rust absent and preserves every third-party file
- **AND** the project configuration, including comments and overrides, stays byte-for-byte identical

#### Scenario: A reader keeps a local fork under the bundled name

- **WHEN** the project contains a locally built `markdown` format plugin with an extra file and a separate `my-markdown`
  plugin
- **AND** the reader runs `ardent update`
- **THEN** the command replaces the complete `markdown` package without prompting, including removing the extra file
- **AND** every file in `my-markdown` retains its contents

#### Scenario: All installed release packages match

- **WHEN** a reader runs `ardent update` again after a successful update
- **THEN** it reports that the installed release plugins are current, exits successfully and changes no files

### Requirement: Keep the previous package usable on update failure

If Ardent cannot read, validate or install a replacement, it SHALL report the affected plugin, return failure and leave
that plugin's previous package usable. A concurrent Ardent sync SHALL see a complete package.

#### Scenario: The bundled component cannot be loaded

- **WHEN** the installed Rust plugin works and the bundled replacement contains an invalid component
- **THEN** `ardent update` fails, names Rust, and the next sync can still use the original Rust package

#### Scenario: Publishing the staged replacement fails

- **WHEN** the candidate package is complete but Ardent cannot put it in the installed location
- **THEN** the command identifies the plugin and its original files remain at the installed location

#### Scenario: Update stops between directory renames

- **WHEN** a filesystem requires two renames and the updater stops after moving the original package to
  `.ardent/plugins/.ardent-update-<name>/`
- **THEN** the next sync, browse or update restores the original before inspecting installed plugins, even if the cache
  has been cleared
- **AND** if the replacement already reached its installed path, Ardent keeps it and completes cleanup

#### Scenario: Only some plugins can be updated

- **WHEN** one replacement fails and another can be installed
- **THEN** the command reports both the successful replacement and the failed plugin and exits with failure

#### Scenario: An interrupted update has conflicting files at the installed path

- **WHEN** the previous package is in the backup and the installed directory contains a different nonempty tree from the
  staged replacement
- **THEN** Ardent preserves both trees, names the backup and continues syncing files handled by healthy formats
- **AND** if the installed directory is empty, Ardent restores the previous package and leaves unrelated backup
  directory entries in place

#### Scenario: A shared Unix package is updated with a restrictive umask

- **WHEN** a reader updates an installed plugin that other project members can read
- **THEN** existing paths retain their sharing permissions and new files and directories inherit the containing
  directory's access policy
- **AND** if the updater cannot preserve an existing owner, it reports failure and leaves the original package installed
- **AND** when update creates `cache/`, `cache/tmp/` or `cache/lock`, each inherits project access

#### Scenario: Another Windows reader recovers a published package

- **WHEN** the updater stops after publishing a replacement and before removing the backup
- **THEN** existing package paths already have their original access rules and new paths inherit their directory's rules
- **AND** a reader with project access can verify the replacement and finish recovery

#### Scenario: A backup contains an unrelated manifest file

- **WHEN** a marked update backup contains an unrelated `plugin.yaml`
- **THEN** sync, browse and update preserve that file and recover the package
