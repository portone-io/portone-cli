# Schema enums

`portone-schema-macros` embeds the original PortOne OpenAPI document. Declare a
type at its point of use; there is no second list of supported enum names:

```rust
use portone_schema_macros::schema_enum;

schema_enum!(pub PaymentStatus);
schema_enum!(pub Currency, cli_case = "preserve");
```

The macro looks up `components.schemas.<Name>` and supports direct, nonempty
string enums. It generates ordinary Rust variants, `clap::ValueEnum`,
`serde::Serialize`, and `as_api_str()`. CLI spellings default to lowercase with
underscores replaced by hyphens. `cli_case = "preserve"` keeps the API spelling;
case-insensitive input, defaults, and repeated values remain clap argument
settings. API serialization always preserves the schema value exactly.

Rust variants use PascalCase. Missing types, unsupported definitions, empty or
duplicate values, invalid variant names, and name collisions produce errors at
the macro invocation. Source order determines help and completion order.

The entire original JSON is committed at
`crates/portone-schema-macros/schema/v2.openapi.json`. It is embedded in the macro
crate with `include_str!` and parsed once per macro process. Builds do not fetch
schemas. Rust expansions are not committed. The initial snapshot comes from
[developers.portone.io commit 1d866f371f561af8408a88e10d7301eb5594502f](https://github.com/portone-io/developers.portone.io/commit/1d866f371f561af8408a88e10d7301eb5594502f).

## Development

To update the schema locally from a sibling checkout's committed `main`:

```sh
git -C ../developers.portone.io show main:src/schema/v2.openapi.json > crates/portone-schema-macros/schema/v2.openapi.json
cargo xtask gen-docs
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo xtask gen-docs --check
cargo vet --locked
node --test scripts/sync-openapi.test.mjs
```

The macro tests discover all named string enums from the schema and compile
them in an isolated consumer project. They also check diagnostic locations and
schema-only incremental rebuilds. Consumer tests cover the CLI and wire formats.

For inspection, install `cargo-expand` and run:

```sh
cargo expand -p portone-cli --lib cmd::payment::schema
cargo expand -p portone-schema-macros --test usage
```

Rust-analyzer's **Expand macro recursively** action also shows the generated code.

## Automatic updates

The `CLI Schema` workflow in `developers.portone.io` runs on committed `main`
changes to `src/schema/v2.openapi.json`, or manually from `main`. It checks out
CLI `main` and calls `scripts/sync-openapi.sh` with the source file and commit SHA.
That script copies the original JSON, regenerates reference docs, and completes
all validation before pushing `chore/openapi` or creating/updating its one PR.
A failed check leaves the remote branch and PR untouched. Identical output does
not create another commit; unrelated API changes can produce schema-only PRs.

Activation order:

1. Merge the Rust CLI, schema macro, and sync script into `portone-cli/main`.
2. Add the `cli-sync` environment to `developers.portone.io`, with a
   `CLI_REPO_TOKEN` secret: a fine-grained PAT scoped to `portone-cli`, with
   **Contents: read/write** and **Pull requests: read/write**. This credential
   lets the resulting PR trigger ordinary CLI CI.
3. Merge the CLI Schema workflow into the documentation repository's `main`.
4. Set its repository variable `CLI_SCHEMA_SYNC_ENABLED` to `true`, then run the
   workflow manually from `main`. Confirm the PR, checks, and an identical rerun.

Keep the variable unset until the prerequisites are ready. Setting it to `false`
pauses synchronization. PR review, merge, and release follow the existing process.
