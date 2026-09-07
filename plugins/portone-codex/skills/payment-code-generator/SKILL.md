---
name: payment-code-generator
description: Implement or migrate PortOne checkout, billing-key, key-in, or identity-verification code using the project's conventions and current official examples from the PortOne MCP server.
---

# PortOne payment code generator

Implement PortOne integrations that fit the existing project rather than
producing standalone sample code.

## Workflow

1. Inspect the repository for its frontend and backend frameworks, dependency
   manager, environment-variable conventions, and existing payment code.
2. Determine the PortOne version and integration type from the request and
   existing code. Ask only when either choice materially changes the result.
3. For V2, retrieve current frontend and backend examples from the PortOne MCP
   server before writing code. For V1, retrieve the relevant official
   documentation and verify every parameter.
4. Adapt the official example to the project's structure, types, error handling,
   and configuration style.
5. Add server-side payment verification, required environment variables, and
   relevant tests.

## Requirements

- Never place an API Secret in client code or commit credentials.
- Do not trust a client-side success result without verifying payment amount
  and status on the server.
- Prefer the current PortOne Server SDK for a V2 backend when the project's
  language is supported.
- Keep `.env` files out of version control.
- When no payment provider is specified, prefer the provider already used by
  the project. Otherwise use Toss Payments only as an example and state that
  assumption.

## Delivery

Return code that can be used directly in the project. Identify changed files,
new dependencies, environment variables, console configuration, and the
shortest useful test procedure.
