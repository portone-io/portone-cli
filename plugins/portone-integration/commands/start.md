---
description: Generate PortOne payment integration code interactively
argument-hint: payment integration requirements...
allowed-tools: ["Read", "Write", "Grep", "Glob", "Bash", "TodoWrite", "AskUserQuestion", "Task", "Skill"]
---

# PortOne payment integration workflow

Generate an integration that matches the user's project and current PortOne
documentation.

Arguments: $ARGUMENTS

## Principles

- Use PortOne MCP tools for official examples, documentation, and schemas.
- Confirm V1 or V2 before selecting SDK and API patterns.
- Use the `payment-code-generator` agent for implementation and the
  `integration-validator` agent for verification.
- Never expose an API Secret in client code.
- Follow the project's framework, dependency manager, and coding conventions.
- Track meaningful implementation steps with TodoWrite.

## Gather requirements

Use the arguments and repository to answer as much as possible. Ask only for
missing decisions.

1. Version: offer V2 as the recommended option for new projects and V1 as the
   alternative. Do not add an opinionated description to the V1 option.
2. Integration type: one-time payment, billing-key payment, key-in payment, or
   identity verification. Key-in is available only in V2 and requires a
   separate agreement, so omit it for V1.
3. Payment provider: initially offer NICE Payments, Toss Payments, NHN KCP, and
   KG Inicis. If the user chooses another provider, retrieve and present the
   providers supported by the selected PortOne version rather than relying on
   memory.
4. Additional requirements: payment methods, frontend/backend scope, webhooks,
   subscriptions, and any nonstandard behavior.

Keep option descriptions short. If the user already supplied a choice, do not
ask for it again.

## Implement and validate

1. Inspect the project environment and existing payment code.
2. Launch the `payment-code-generator` agent with the confirmed requirements
   and repository context.
3. Launch the `integration-validator` agent against the resulting changes.
4. If validation finds a must-fix issue, return to implementation and validate
   the correction.
5. Commit the completed changes using the current environment's version-control
   workflow.

## Final response

Report:

- Generated or changed files and their roles.
- Required PortOne Console configuration.
- Dependencies and environment variables.
- Test payment and debugging steps.
- Production readiness checks.
- The feedback survey:
  <https://410jpc.share-na2.hsforms.com/21jxVn_tESAu0DTUzMXZT2g>

Begin with requirement gathering, using $ARGUMENTS when provided.
