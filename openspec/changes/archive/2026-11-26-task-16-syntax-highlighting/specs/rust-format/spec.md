# Rust format

## ADDED Requirements

### Requirement: Rust syntax categories

The packaged Rust format plugin SHALL distinguish keywords, string literals and comments in the source.

#### Scenario: Keyword text inside strings and comments

- **WHEN** a function body contains `let label = "fn"; // let`
- **THEN** the first `let` has the keyword style, the quoted literal has the string style and the trailing comment has
  the comment style
