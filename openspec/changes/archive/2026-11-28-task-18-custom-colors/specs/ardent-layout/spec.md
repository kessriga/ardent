# Theme files

## ADDED Requirements

### Requirement: Select a named theme

The optional `theme` key in project and user `config.yaml` SHALL select a filename stem. The project selector SHALL win
over the user selector. With neither selector the browser SHALL use the built-in palette. Ardent SHALL reject empty
names, `.` and `..`, path separators and absolute paths before looking up a theme.

#### Scenario: Project selects a different theme

- **WHEN** user settings select `daylight` and project settings select `catppuccin`
- **THEN** the browser loads `catppuccin.yaml` from the user's Ardent theme directory

### Requirement: Resolve colors within the selected theme

Ardent SHALL look up the selected name only in `$XDG_CONFIG_HOME/ardent/themes/<name>.yaml`, using the existing XDG
configuration home rules. The theme mapping SHALL contain `schema: 1` in any key order. Each omitted source category or
outline kind SHALL use its built-in color. Reading a theme SHALL leave the directory and file untouched. The browser
SHALL reject theme files larger than 1 MiB and report the limit and file path.

#### Scenario: A checkout contains a theme of the same name

- **WHEN** both the user theme directory and a project's `.ardent/themes/` contain `catppuccin.yaml`, and the selected
  name is `catppuccin`
- **THEN** the browser uses only the installed user file
- **AND** an outline kind omitted from that file uses its built-in color even if the project copy supplies it

#### Scenario: Restore the palette

- **WHEN** the reader removes the last effective `theme` selector while browsing
- **THEN** source highlights and outline labels use their built-in colors and the saved theme files remain available

### Requirement: Source and outline entries

The optional `source` map SHALL accept `keyword`, `string_literal`, `comment`, `heading` and `code`. The optional
`outline` map SHALL accept exact node kind names, including plugin-defined kinds and the built-in `file` kind. Values
SHALL be strings of the form `#RRGGBB`, accepting upper- or lowercase hexadecimal digits.

#### Scenario: Configure a function and a comment

- **WHEN** a reader sets `outline.fn` to `"#88CCFF"` and `source.comment` to `"#B0B0B0"` and opens a Rust file
- **THEN** function labels and highlighted comments use their respective colors

### Requirement: Report a theme lookup or parse error

The browser SHALL name the file and cause when an active theme file is unreadable or invalid. If the selected theme is
missing from the user theme directory, the diagnostic SHALL identify the expected path.

#### Scenario: A project selects a theme the reader has not installed

- **WHEN** the selected theme exists only in the checkout
- **THEN** the browser reports the missing file in the user theme directory

#### Scenario: Mistyped color

- **WHEN** a reader sets `source.comment` to `"#GG1020"` in the user theme and opens the browser
- **THEN** Ardent names the theme file and reports the invalid comment setting
- **AND** the reader can still run `ardent sync`
