---
"@portone/cli": minor
---

Rewrite the CLI in Rust and add `portone api` and `portone auth`.

- `portone api <endpoint>` provides a general-purpose, `gh api`-style PortOne V2 API client with `-F`/`-f` fields, embedded jaq through `--jq`, automatic offset or cursor pagination, `--slurp`, `--cache`, `--include`, and `--verbose`.
- `portone auth login/status/logout` validates API Secrets and stores profiles in `~/.config/portone/config.toml`.
- Distribution moves to platform-specific native binaries (`@portone/cli-<platform>`), removes the Node.js runtime dependency, and preserves the existing `portone setup` interface.
