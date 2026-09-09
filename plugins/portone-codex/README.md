# PortOne Codex Plugin

A Codex plugin for implementing and reviewing PortOne payment integrations.

## Features

- Generate PortOne V1 and V2 payment integration code.
- Validate existing integrations and diagnose concrete problems.
- Use official PortOne documentation and MCP examples as the source of truth.
- Use the PortOne CLI for authentication, payment inspection, and API requests.

## Installation

Install Codex, Git, Node.js, and `npx`, then run:

```bash
portone setup --assistant codex
```

Setup installs `portone-codex@portone` for your user account through Codex's
plugin manager. Running it again refreshes the marketplace and updates the
plugin. It does not install or update Codex itself.

The plugin includes its MCP configuration. Its bundled `.mcp.json` uses:

```json
{
  "mcpServers": {
    "portone": {
      "type": "stdio",
      "command": "npx",
      "args": ["-y", "@portone/mcp-server@latest"]
    }
  }
}
```

Start a new Codex session after setup, check the PortOne server with `/mcp`,
and ask Codex to retrieve a PortOne document. If setup reports that the plugin
is inactive, enable it in `/plugins` and rerun setup. Console features may
request login when used; setup does not log in or save tokens.

## Usage

Ask Codex for the integration work you need:

```text
Implement a PortOne V2 one-time payment integration.
Review the PortOne integration in this project.
Add a PortOne billing-key payment flow.
Use the PortOne CLI to inspect failed test payments.
```

## Included skills

- `payment-code-generator`: implement a new PortOne integration.
- `integration-validator`: validate an existing or newly generated integration.
- `portone-guide`: explain PortOne concepts and locate official guidance.
- `portone-cli`: authenticate and use PortOne CLI payment and API commands.

## Maintaining the CLI skill

`skills/portone-cli/` is a generated copy of the repository's root
`skills/portone-cli/`. Edit the root source, run `cargo xtask sync-plugin-skills`,
and commit both plugin copies. Use `cargo xtask sync-plugin-skills --check` to
verify they are current.

## License

MIT License
