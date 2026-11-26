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
    rumdl fmt --no-cache --enable MD013 --exclude "crates/ardent-format/tests/markdown/**" "$@"

lint:
    cargo clippy --locked --workspace --all-targets -- -D warnings

test:
    cargo test --locked --workspace

build:
    cargo build --locked --workspace

test-recipes:
    python3 scripts/test_recipes.py

# Run the Rust workspace and recipe checks.
check: fmt-check lint test build test-recipes

# Install the release binary and its bundled format plugins.
[positional-arguments]
install root="":
    python3 scripts/install.py "$1"
