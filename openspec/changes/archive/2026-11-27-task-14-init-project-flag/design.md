# Design

Decision-8 defines the project and user package locations. In `ardent-cli`, the scripted path already builds a project
selection from `--plugins`. Declare `--project` on the init subcommand and require `--plugins` through the argument
parser. Both scripted spellings can then use the same installation path. The installer, project config and plugin
verification need no new types or branches.
