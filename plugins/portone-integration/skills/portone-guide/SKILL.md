---
name: portone-guide
description: Guide PortOne V1 or V2 payment and identity-verification integrations using current official documentation, API schemas, SDK examples, and PortOne MCP tools.
---

# PortOne integration guide

Use this skill for PortOne concepts, architecture choices, and documentation
discovery. Use the payment code generator for implementation and the integration
validator for review when those capabilities are available.

## Choose the integration

Prefer V2 for a new project. Follow the version already used by an existing
project unless the user requests a migration or a required provider capability
dictates otherwise. V1 and V2 use different APIs and SDK patterns; never mix
them within one flow.

Common integration types:

- One-time payment: open a payment provider checkout window for an individual
  card, bank transfer, virtual-account, mobile, or easy-pay purchase.
- Billing-key payment: issue a billing key, store it securely on the server, and
  charge it for subscriptions or other server-initiated payments.
- Key-in payment: submit card details through an approved server-side flow. V2
  availability requires a separate contract.
- Identity verification: verify a user during signup, age checks, or other
  identity-sensitive workflows.

If the version, payment type, payment provider, or required features cannot be
derived from the request and repository, ask only for the missing decisions.

## Use official sources

Use the PortOne MCP server before writing or validating integration code.

- Retrieve V2 frontend examples for React or HTML with the requested provider
  and payment method.
- Retrieve V2 backend examples for Express, FastAPI, Flask, or Spring/Kotlin.
- Search and read official documentation for V1 flows and provider-specific
  behavior.
- Inspect the V1 or V2 OpenAPI schema for exact paths, fields, and enum values.
- Retrieve merchant, channel, or shared test-channel information only when the
  task requires it.

Do not guess API fields or copy an example for a different PortOne version,
provider, payment method, or SDK release.

## Implementation flow

1. Inspect the repository for frameworks, dependencies, existing PortOne code,
   environment variables, and testing conventions.
2. Confirm the PortOne version, integration type, payment provider, payment
   method, and any nonstandard requirements.
3. Retrieve the matching official examples and documentation.
4. Adapt the examples to the project's structure and coding style.
5. Implement server-side verification and webhook handling where applicable.
6. Test the flow with a PortOne test channel and report required console setup.

## Frontend guidance

Load the V2 browser SDK from the official package and use the API shape returned
by the current MCP example. Generate a unique payment identifier on the server
or according to the application's trusted order flow. Keep store and channel
identifiers configurable.

Handle user cancellation, provider errors, and network errors separately when
the SDK exposes that distinction. A successful browser response is not proof
that the order is paid.

## Backend guidance

Keep credentials on the server and prefer a supported PortOne Server SDK for V2.
After the browser flow completes:

1. Retrieve the payment from PortOne.
2. Verify its status, amount, currency, store, and order ownership.
3. Update the order atomically and idempotently.
4. Reject duplicate, mismatched, cancelled, or failed payments.

For billing-key flows, treat billing keys as sensitive data and initiate charges
only from an authenticated, authorized server context.

## Webhooks

Register an HTTPS webhook URL in PortOne Console. Verify webhook authenticity
using the current PortOne documentation, tolerate retries and out-of-order
delivery, and make processing idempotent. Fetch the authoritative resource from
PortOne before changing business state when the event payload is not sufficient.

## Security requirements

- Never expose an API Secret, billing key, or private channel credential in
  browser code, logs, commits, or error responses.
- Store server credentials in environment variables or an approved secret
  manager.
- Do not commit `.env` files.
- Validate amount and payment status on the server before fulfilling an order.
- Authorize access to order and payment identifiers; do not trust client input.
- Verify webhooks and defend against duplicate delivery.

## Testing

Use test channels and provider-specific test data from official documentation.
Cover success, user cancellation, declined payment, amount mismatch, duplicate
completion, server verification failure, webhook retries, and invalid webhook
authentication.

Before production, confirm live channel configuration, production credentials,
allowed origins, webhook reachability, observability, and recovery behavior.
