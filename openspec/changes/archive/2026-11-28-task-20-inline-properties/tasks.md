# Tasks

## 1. Format plugin properties

- [x] 1.1 Add the optional property interface and host validation. Document it in `guests/README.md`. Test opted-in and
      hidden values, absent exports, traps, invalid indices, and duplicate groups or names. Verify the existing
      scanner-only fixture still loads.
- [x] 1.2 Load the open file’s properties in the CLI and pass them to the browser. Test a source edit, a changed
      component and an unavailable property export. Verify values belong to the matching document revision, including
      when a list and its item share a range.

## 2. Packaged formats

- [x] 2.1 Emit Markdown checkbox properties from the item’s own marker. Test checked, unchecked, plain and nested items
      with the native guest suite.
- [x] 2.2 Extend the Rust scan to include generics, inputs, receivers and explicit returns in definitions and
      signatures. Assert node paths and exact preview slices for explicit unit, omitted return, typed and shorthand
      receivers, and a function-pointer input. Review the resulting Rust outline snapshots.
- [x] 2.3 Emit applicable Rust visibility, type and bound properties. Test a public static, a restricted declaration, a
      private field, an enum variant, a type parameter and a const generic. Verify existing function paths and
      fingerprints in the Rust example.
- [x] 2.4 Rebuild and stage both release packages using `guests/README.md`. Run the native guest and packaging tests.
      Verify an update of an installed older package yields the new outline and properties on the same source.

## 3. Outline interaction

- [x] 3.1 Render indented rows before children and maintain disclosure per node across navigation and refresh. Test
      multiple open nodes, empty properties, hidden values, terminal control characters and selection below a block of
      properties.
- [x] 3.2 Bind Tab to properties in the outline and Shift+Tab to pane switching. Update the browser hints and README
      reader guide. Test both directions at wide and narrow widths and assert the selected node, child expansion and
      preview scroll survive the toggle.

## 4. Integration

- [x] 4.1 Run `just check`, format changed documentation with `just fmt-md` excluding Markdown snapshot inputs, and run
      `openspec validate --all --strict`. Read the updated task back with `backlog task view TASK-20 --plain`.

## Workflow follow-up

- Archive the change in the implementation PR and update TASK-20 with the verification results.
