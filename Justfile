cargo_root := env("CARGO_INSTALL_ROOT", env("CARGO_HOME", home_directory() / ".cargo"))

default:
    @just --list

# Format the Rust workspace.
fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all --check

# Format the specified Markdown files.
[positional-arguments]
fmt-md +files:
    rumdl fmt --no-cache --enable MD013 "$@"

lint:
    cargo clippy --locked --workspace --all-targets -- -D warnings

test:
    cargo test --locked --workspace

build:
    cargo build --locked --workspace

# Run the Rust workspace checks.
check: fmt-check lint test build

# Install the release binary and its bundled format plugins.
[positional-arguments]
install root=cargo_root:
    cargo install --locked --path crates/ardent-cli --force --root "$1"
    mkdir -p "$1/bin/plugins"
    cp -R plugins/. "$1/bin/plugins/"
