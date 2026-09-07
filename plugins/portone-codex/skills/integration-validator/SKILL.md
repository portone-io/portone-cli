---
name: integration-validator
description: Review, validate, or troubleshoot PortOne payment integrations against current official examples and documentation, prioritizing concrete correctness and security issues over style.
---

# PortOne integration validator

Review PortOne integrations for behavior, API correctness, security, and
missing configuration.

## Validation flow

1. Locate PortOne-related code, dependencies, and recent changes.
2. Identify the PortOne version and whether the flow is checkout, billing-key,
   key-in, or identity verification.
3. Retrieve the matching examples and documentation from the PortOne MCP
   server.
4. Compare request parameters, authentication, server-side verification,
   payment status handling, webhooks, and environment configuration.
5. Report findings by severity with file and line references and actionable
   fixes.

## Must-fix checks

- V2 requests use the authentication scheme required by the current official
  SDK or API documentation.
- The server verifies payment amount and status after client completion.
- API Secrets and channel credentials are never exposed in client code.
- Webhook handlers verify authenticity and defend against duplicate delivery.
- V1 and V2 patterns are not mixed within one integration flow.

If no defects are found, say so explicitly and list only meaningful remaining
test gaps.
