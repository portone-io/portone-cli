---
name: portone-cli
description: Use the PortOne CLI for non-interactive authentication and PortOne V2 REST or GraphQL requests, including typed fields, pagination, jq filtering, caching, output handling, and schema discovery.
---

# Using `portone`

`portone` provides authenticated PortOne V2 REST and GraphQL requests through
`portone api`. It is safe to call from an agent or CI environment: API commands
never prompt, and non-TTY output disables color and paging automatically.

## Non-interactive behavior

- Do not look for a `--no-pager` flag; it does not exist and is unnecessary.
- Set `CLICOLOR_FORCE=1` to force color or `NO_COLOR=1` to disable it.
- `portone setup` requires `--assistant claude|codex|both` when no TTY is
  available.
- `portone auth login --no-browser` prints a login URL to stderr and waits up to
  five minutes for the OAuth callback. Give that URL to the user so they can
  complete login in a browser.

## Authentication

Built-in authentication uses a PortOne Console OAuth token and sends
`Authorization: Bearer <token>` for both REST and GraphQL requests.

Credential precedence is:

1. `PORTONE_ACCESS_TOKEN`.
2. The selected OAuth profile: `--profile`, then `default_profile`, then
   `default`.

Use `PORTONE_ACCESS_TOKEN` in CI or agent environments that already have a
console token. Tokens supplied through the environment are not refreshed.
`auth login` and `auth logout` refuse to run while this variable is set.

For an interactive account:

```bash
portone auth login --no-browser
portone auth status
portone auth token
portone auth logout
```

Login stores tokens in the OS keyring, or in the config file when
`--insecure-storage` is passed or the keyring is unavailable. Stored access
tokens refresh automatically. `auth token` prints a fresh access token for
another tool, for example:

```bash
PORTONE_ACCESS_TOKEN=$(portone auth token) npx @portone/mcp-server
```

The API base URL is resolved in this order: `--base-url`, `PORTONE_API_BASE`,
the profile's `base_url`, then `https://api.portone.io`.

Pass `-H 'Authorization: ...'` to override built-in authentication. When an
endpoint is a full URL with a different origin from the base URL, the CLI does
not attach Authorization.

## REST requests

`portone api` is the only command that sends API requests; there are no typed
resource commands such as `payments list`.

```bash
portone api /payments/{paymentId}
portone api '/payments/{paymentId}?storeId=store-xxx'
```

Replace endpoint placeholders with actual values. The method defaults to GET
and switches to POST when `-F`, `-f`, or `--input` supplies a body. Override it
with `-X`.

PortOne V2 list endpoints such as `GET /payments` and
`GET /payment-schedules` accept `page` and `filter` in a GET request body.
Always pass `-X GET` when sending fields to these endpoints:

```bash
portone api /payments -X GET -F 'page[size]=10' -F 'filter[isTest]=true'
```

## Request fields

- `-f key=value` always sends a string.
- `-F key=value` converts integers, `true`, `false`, and `null` to JSON types.
  Decimal values such as `1.5` remain strings.
- With `-F`, `@path` reads a value from a file and `@-` reads from standard
  input.
- Use `key[sub]=value` for objects, repeated `key[]=value` fields for arrays,
  `key[][sub]=value` for arrays of objects, and `key[]` without a value for an
  empty array. Repeating a scalar key is an error.
- Use `--input file.json` or `--input -` for a complete request body.
  `--input` cannot be combined with field flags or `--paginate`.

## Pagination

A request returns one page unless `--paginate` is set.

- Offset pagination starts with `page.number=0` and `page.size=100` when the
  request has no `page` field, then uses `page.totalCount`. It stops at offset
  60000. Use a cursor endpoint such as `/payments-by-cursor` for larger result
  sets.
- Cursor pagination is selected when the request has `size` or `cursor`, or the
  response contains `items[].cursor`. The default size is 1000.
- Each page is printed as a separate JSON value. Add `--slurp` to wrap all
  pages in one array, or filter values directly with a command such as
  `--paginate -q '.items[].id'`.
- REST pagination only supports GET and cannot be combined with `--input`.
- If the response does not reveal a pagination scheme, the CLI prints the
  first page and warns on stderr before stopping.

## GraphQL

Use `graphql` as the endpoint:

```bash
portone api graphql -f query='query { merchant { ... on Merchant { id plainId } } }'
```

Every `-F` or `-f` field other than `query` and `operationName` becomes a
GraphQL variable. Build object variables with nested fields:

```bash
-F 'filter[statuses][]=IN_PROGRESS' -F 'filter[cardCompanies][]'
```

The schema root exposes `node(id: ID!)` and `merchant`. Results are unions, so
queries generally need inline fragments such as `... on Merchant { ... }`.
Include `__typename` when a result appears empty; it may be an error type such
as `ForbiddenError`. Store fields can return `ForbiddenError` when the token
lacks access, while `node(id:)` may return `null` for an inaccessible object.

GraphQL `id` values are global IDs returned by GraphQL, not plain IDs from REST.
Do not construct them manually; reuse the exact `id` value from a response.

A response containing `errors` exits with status 1 even when the HTTP status is
200. The original JSON is written to stdout and the message to stderr.

GraphQL pagination supports Relay-style cursors only. The query must accept an
`$endCursor: String` variable and select
`pageInfo { hasNextPage endCursor }`. Offset-style GraphQL fields must be
iterated manually. GraphQL requests always use POST and may be cached.

## Output and caching

- Non-TTY JSON is emitted with the server's bytes unchanged.
- `-q/--jq` uses embedded jaq. Most jq syntax works, but some built-ins may be
  unavailable or differ slightly.
- Only one of `--jq`, `--silent`, or `--verbose` may be used. `--include` adds
  the response status and headers. `--verbose` prints the request and response
  while masking Authorization.
- Non-JSON responses containing terminal escape sequences are rejected unless
  `--allow-escape-sequences` is set.
- `--cache 1h` caches matching GET, HEAD, and GraphQL requests. Responses with
  a 403 or 5xx status are not cached. Set `PORTONE_CACHE_DIR` to override the
  platform cache directory.

## Schema discovery

Never guess endpoint paths, field names, or enum values. Check the current
schema first:

- REST OpenAPI: `https://developers.portone.io/schema/v2.openapi.json`
- GraphQL SDL: `https://developers.portone.io/schema/v2.graphql`
- When `@portone/mcp-server` is available, use
  `readPortoneOpenapiSchemaSummary` and `readPortoneOpenapiSchema`.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Success, including an output pipe that closes early |
| 1 | HTTP or GraphQL error, missing credentials, invalid flag combination, or runtime error |
| 2 | Command-line argument parsing error |

Errors are written to stderr as `portone: <message>`. HTTP response bodies are
written to stdout.
