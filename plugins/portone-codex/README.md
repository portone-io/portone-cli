# PortOne Codex Plugin

A Codex plugin for implementing and reviewing PortOne payment integrations.

## Features

- Generate PortOne V1 and V2 payment integration code.
- Validate existing integrations and diagnose concrete problems.
- Use official PortOne documentation and MCP examples as the source of truth.

## Prerequisite

The plugin requires the `@portone/mcp-server` MCP server. Its bundled
`.mcp.json` uses this default configuration:

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

## Usage

Ask Codex for the integration work you need:

```text
Implement a PortOne V2 one-time payment integration.
Review the PortOne integration in this project.
Add a PortOne billing-key payment flow.
```

## Included skills

- `payment-code-generator`: implement a new PortOne integration.
- `integration-validator`: validate an existing or newly generated integration.
- `portone-guide`: explain PortOne concepts and locate official guidance.

## License

MIT License
