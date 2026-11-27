# Proposal

## Why

Reinstalling Ardent updates the release packages beside the executable. Existing projects keep their installed copies,
and a rebuilt component can have the same manifest version. TASK-19 gives readers a way to see and apply the difference.

## What Changes

- Show a notice in the browser naming installed release plugins that differ from the running release.
- Add `ardent update` to replace each such package on request, including its documentation and other packaged files.
- Keep each project's installation choices, format settings and third-party packages. Network downloads, binary upgrades
  and third-party plugin management are outside this task.

## Capabilities

### Modified Capabilities

- `ardent-layout` gains rules for updating installed release packages and preserving the previous package on failure.
- `outline-browser` gains the update notice and clears it when the installed copies match.

## Impact

The CLI owns release discovery, the new command and the browser's update check. The terminal browser displays the
notice. The README will explain when to run the command and how it treats local edits to a bundled package.
