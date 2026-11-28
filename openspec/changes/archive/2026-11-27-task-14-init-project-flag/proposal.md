# Proposal

## Why

TASK-14 lets a person choose an installation scope during interactive init. Scripts need a way to state that the bundled
format plugins belong in the project.

## What Changes

Accept `ardent init --plugins markdown,rust --project`. With this flag, init installs the named packages in the project
and enables them without prompting. Show the flag in `ardent init --help` and document the scripted invocation.

## Capabilities

### Modified Capabilities

- `ardent-layout` specifies the project destination for an explicit scripted installation.

## Impact

The change affects CLI argument parsing, init tests and the README. Interactive prompts, the package layout, and
compilation cache policy are outside its scope.
