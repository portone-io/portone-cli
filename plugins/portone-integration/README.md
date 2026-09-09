# PortOne Integration Plugin

A Claude Code plugin for implementing and reviewing PortOne payment
integrations.

## Features

- Generate PortOne V1 and V2 integration code for supported frontend and
  backend frameworks.
- Support one-time payments, billing-key payments, key-in payments, and
  identity verification.
- Review existing integrations for security, correctness, and PortOne best
  practices.
- Use the PortOne CLI to authenticate, inspect payments, and make API requests
  with the bundled `portone-cli` skill.

## Installation

Install Claude Code, Git, Node.js, and `npx`, then run:

```bash
portone setup --assistant claude
```

Setup installs `portone-integration@portone` for your user account through
Claude Code's plugin manager. Running it again refreshes the marketplace and
updates the plugin. It does not install or update Claude Code itself.

The plugin includes its MCP configuration, which runs
`npx -y @portone/mcp-server@latest`. Start a new Claude Code session, check the
PortOne server with `/mcp`, and ask Claude to retrieve a PortOne document.
Console features may request login when used; setup does not log in or save
tokens.

## Usage

### `/start`

Generate payment integration code interactively:

```text
/portone-integration:start
/portone-integration:start v2
/portone-integration:start v2 checkout
/portone-integration:start v1 billing
```

Payment types:

- `checkout`: one-time payment through a payment provider checkout window.
- `billing`: recurring or on-demand payments using a billing key.
- `keyin`: payment using card details entered directly.
- `identity`: identity verification.

### Review an integration

Ask Claude to review an existing PortOne integration. The plugin's
`integration-validator` agent handles requests such as:

```text
Review the PortOne integration in src/payment/ for security issues.
Validate the PortOne API calls in src/api/pay.ts.
```

The plugin also activates its specialized agents and skills for natural
language requests such as:

```text
Implement PortOne payment support.
Add a recurring payment integration.
Review this PortOne integration for security issues.
Use the PortOne CLI to inspect failed test payments.
```

## Maintaining the CLI skill

`skills/portone-cli/` is a generated copy of the repository's root
`skills/portone-cli/`. Edit the root source, run `cargo xtask sync-plugin-skills`,
and commit both plugin copies. Use `cargo xtask sync-plugin-skills --check` to
verify they are current.

## Supported frameworks

Frontend examples cover React, vanilla HTML/JavaScript, and Vue adaptations.
Backend examples cover Express, FastAPI, Flask, and Spring with Kotlin.

## Choosing an integration

- Use one-time payments for individual purchases completed in a payment
  provider checkout window.
- Use billing-key payments for subscriptions, memberships, and server-initiated
  charges.
- Use identity verification for signup, age checks, and similar flows.
- Prefer V2 for new projects. Use V1 when maintaining an existing V1
  integration or when a required provider feature is only available in V1.

## Security

- Never expose an API Secret in client code.
- Verify completed payments on the server.
- Keep credentials in environment variables and exclude `.env` files from
  version control.

## License

MIT License
