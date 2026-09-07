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

## Prerequisite

The plugin requires the `@portone/mcp-server` MCP server. Add it to the
project's `.mcp.json` file:

```json
{
  "mcpServers": {
    "portone": {
      "type": "stdio",
      "command": "npx",
      "args": ["@portone/mcp-server@latest"]
    }
  }
}
```

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

### `/review`

Review an existing PortOne integration:

```text
/portone-integration:review
/portone-integration:review src/payment/
/portone-integration:review src/api/pay.ts
```

The plugin also activates its specialized agents and skills for natural
language requests such as:

```text
Implement PortOne payment support.
Add a recurring payment integration.
Review this PortOne integration for security issues.
```

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
