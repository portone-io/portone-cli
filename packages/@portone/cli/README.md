# portone-cli

[![npm version](https://img.shields.io/npm/v/%40portone%2Fcli)](https://www.npmjs.com/package/@portone/cli)
[![license](https://img.shields.io/github/license/portone-io/portone-cli)](https://github.com/portone-io/portone-cli/blob/main/LICENSE)

PortOne CLI helps you integrate PortOne payments and identity verification. It
provides payment search, inspection, cancellation, and webhook management
(`portone payment`), authenticated PortOne V2 API requests (`portone api`),
authentication management (`portone auth`), and PortOne plugin setup for Claude
Code and Codex (`portone setup`).

- Repository: <https://github.com/portone-io/portone-cli>
- Issues: <https://github.com/portone-io/portone-cli/issues>

## Installation

```bash
npm install --global @portone/cli
```

Node.js 22.18.0 or later in the 22.x line, or Node.js 24 or later, is required.

## Display language

The CLI includes English and Korean translations. It detects your language
automatically and falls back to English when no supported language is found.
Set `PORTONE_LANG` for one process, or the top-level `language` setting in
`config.toml` to save a preference:

```bash
PORTONE_LANG=en portone auth status
PORTONE_LANG=ko portone --help
```

```toml
language = "ko" # en, ko, or auto (the default)
```

`PORTONE_LANG` takes precedence over the saved preference. Setting it to `auto`
bypasses the saved preference. Empty values are treated as unset; unsupported
values fall back to English. Regional locales such as `ko-KR` and
`ko_KR.UTF-8` are also recognized.

After explicit PortOne language settings, automatic detection uses the operating
system's preferred languages. On macOS and Windows, these come from the OS UI
language preferences. On Linux, language candidates are collected from
`LANGUAGE`, `LC_ALL`, `LC_MESSAGES`, and `LANG`, in that order. `LANGUAGE` accepts
a colon-separated preference list. The first supported language is selected,
with English as the fallback. Use `PORTONE_LANG=en` to force English on any
platform.

Help, prompts, authentication status, and CLI diagnostics use the selected
language. Command names, flags, API responses, tokens, timestamps, and external
error details are unchanged. Argument-parsing diagnostics from clap remain in
English. Generated completion scripts and reference documentation use English
regardless of the selected language.

For agent or CI invocations, set `PORTONE_LANG=en` for consistent diagnostics
without changing the user's saved preference. On PowerShell, set the process
environment with `$env:PORTONE_LANG = 'en'` before invoking the CLI.

## `portone setup`

Install the PortOne plugins for Claude Code, Codex, or both:

```bash
portone setup
portone setup --assistant claude
portone setup --assistant codex
portone setup --assistant both
```

Setup uses each assistant's plugin manager to install from the `portone`
marketplace at `portone-io/portone-cli`. Claude Code receives
`portone-integration@portone`; Codex receives `portone-codex@portone`. Both are
installed for your user account and available across projects. Running setup
again refreshes the marketplace and updates the plugin.

Install your selected assistants, Git, Node.js, and `npx` first. The assistant
versions must support the plugin commands used by setup; setup checks all
selected assistants and prerequisites before making changes. It does not
install or update Claude Code or Codex itself. Noninteractive invocations
must specify `--assistant`.

Each plugin includes the `portone-cli` skill for CLI authentication, payment
inspection, and API requests, along with the existing payment integration
skills or agents. The bundled MCP configuration starts
`npx -y @portone/mcp-server@latest` when the assistant loads the plugin. Start a
new assistant session after setup, check the PortOne server with `/mcp`, and
ask it to retrieve a PortOne document. Console features may request login when
used; setup itself does not log in or save tokens.

Setup verifies installation and activation before reporting success. If Codex
reports an inactive plugin, enable it in `/plugins` and rerun setup. If one
assistant fails, setup exits with code 1 and identifies completed and failed
targets so you can retry the failed target.

Setup works outside Git repositories and leaves project files unchanged. The
deprecated `--allow-dirty` flag is accepted as a hidden no-op for compatibility
with existing scripts.

### Existing marketplace conflicts

If the name `portone` is already registered to another source, setup stops
without replacing it. Inspect the source with `claude plugin marketplace list
--json` or `codex plugin marketplace list --json`. Keep the existing registration
if you need it, or remove that registration through the assistant's plugin
manager after checking which installed plugins depend on it. Then rerun
`portone setup --assistant claude` or `portone setup --assistant codex` to register
`portone-io/portone-cli`.

### Maintaining the bundled CLI skill

In the source repository, edit `skills/portone-cli/` and synchronize its copies
into both plugin bundles:

```bash
cargo xtask sync-plugin-skills
cargo xtask sync-plugin-skills --check
```

Commit the generated copies with the source changes. CI checks for missing,
changed, or stale generated files. Publish the plugin and marketplace changes
to the GitHub default branch before releasing a CLI version that requires them.

## `portone auth`

Manage credentials and profiles. Built-in authentication uses PortOne Console
OAuth and sends `Authorization: Bearer <token>` for both REST and GraphQL
requests.

```bash
portone auth login                # Authenticate through PortOne Console in a browser
portone auth login --profile staging --base-url <URL>
portone auth login --no-browser --scopes TX_READ,STORE_READ
portone auth status               # Show the source, expiry, scopes, and validity
portone auth status --show-secret # Show the unmasked access token
portone auth token                # Print the current access token, refreshing if needed
portone auth logout               # Remove local credentials without revoking the token
```

`login` validates the issued token before saving it. On first login, it also
selects the merchant's representative store as the profile's default store and
prints its name and ID. Reauthenticating retains a previously selected default
when it is still accessible. If no representative store is available, a sole
accessible store is selected automatically; multiple stores can be selected
interactively or skipped. Failure to discover stores does not prevent login.

### Console login

`auth login` starts a callback server on `127.0.0.1:1271` and opens the PortOne
Console login page. Set `PORTONE_BROWSER` or `BROWSER` to choose the browser, or
pass `--no-browser` to print the URL without opening it. The command exchanges
the callback code, validates the token through GraphQL, and saves the
credentials. It stops if no callback arrives within five minutes.

- Access tokens are refreshed 60 seconds before expiry by `portone api`,
  `auth status`, and `auth token`. An interprocess lock serializes refreshes on
  the same machine.
- Refresh tokens rotate on use and expire after 24 hours of inactivity. Run
  `portone auth login` again after the session expires.
- Log in separately on each machine. Copying a profile between machines can
  invalidate one session when the other rotates its refresh token.
- Tokens are stored in the OS keyring as `portone-cli/<credential_id>` (macOS
  Keychain, Windows Credential Manager, or Linux Secret Service). If the
  keyring is unavailable, the CLI warns and falls back to the config file. Use
  `--insecure-storage` to choose file storage explicitly.
- A profile retains the console URL, token endpoint, and API base URL from the
  issuing environment so later commands use the same environment.

Use `auth token` to pass a token to another tool such as the PortOne MCP server:

```bash
PORTONE_ACCESS_TOKEN=$(portone auth token) npx @portone/mcp-server
```

### Credential precedence

1. `PORTONE_ACCESS_TOKEN`, used as provided and never refreshed by the CLI.
2. An OAuth profile from the config file: `--profile`, then `default_profile`,
   then `default`.

`auth login` and `auth logout` refuse to run while `PORTONE_ACCESS_TOKEN` is
set. Unset it before changing stored credentials.

The API base URL is resolved independently in this order: `--base-url`,
`PORTONE_API_BASE`, the selected profile's `base_url`, then
`https://api.portone.io`.

### Configuration

The config file is stored at `~/.config/portone/config.toml` on Unix-like
systems and `%APPDATA%\portone\config.toml` on Windows. Set
`PORTONE_CONFIG_DIR` to use another directory. The file contains sensitive
authentication metadata and is written with owner-only permissions (`0600`).

Use profiles to separate credentials for different merchants or environments,
and select one with `--profile <NAME>`:

```toml
default_profile = "default"

[profiles.default]
base_url = "https://api.portone.io"
store_id = "store-xxx"

[profiles.default.oauth]
storage = "keyring"          # Tokens are stored as portone-cli/<credential_id>
credential_id = "..."
client_id = "CLI"
token_url = "https://merchant-service.prod.iamport.co/oauth/token"
console_url = "https://admin.portone.io"
```

The login environment can be overridden with `PORTONE_CONSOLE_URL`,
`PORTONE_MERCHANT_SERVICE_URL`, `PORTONE_OAUTH_CLIENT_ID`, and
`PORTONE_OAUTH_REDIRECT_URI`. These variables only affect login.

## `portone store`

Manage the default store saved in a profile:

```bash
portone store set-default                    # Select an accessible store
portone store set-default store-xxx          # Save an ID directly
portone store set-default --profile staging --view
portone store set-default --unset
```

The selector shows store names and IDs with the representative store first.
This changes only the CLI profile, not the representative store configured in
PortOne. `--view` displays the stored value, without environment overrides.

## `portone payment`

Search payments, inspect failures and payment attempts, cancel payments, and
inspect or resend webhooks:

```bash
portone payment list --test --status failed --limit 20
portone payment view payment-xxx
portone payment transactions payment-xxx
portone payment webhook list payment-xxx
portone payment cancel payment-xxx --reason 'Customer request'
portone payment cancel payment-xxx --amount 1000 --reason 'Partial refund' --yes
portone payment webhook resend payment-xxx --webhook-id webhook-xxx
```

Payment IDs are the IDs assigned by your integration. Use `list --search TEXT`
to find a payment by its PortOne or PG transaction ID.

### Store and search defaults

Store selection follows `--store`, `PORTONE_STORE_ID`, the profile's `store_id`,
then the API default. `payment list --all-stores` ignores the environment and
profile defaults and omits the store filter. It cannot be combined with
`--store`; the accessible result range depends on the token.

`payment list` returns the newest 30 V2 payments changed within the past 90
days, including both test and live payments. `--limit/-L` sets the final number
of results from 1 to 60,000, with pages fetched automatically.

```bash
portone payment list --live --status paid,partial-cancelled --currency KRW
portone payment list --method card --pg tosspayments --version all
portone payment list --from 2026-09-01T00:00:00+09:00 --until 2026-09-08T00:00:00+09:00
portone payment list --search payment-xxx --search-field payment-id
```

Use `--time-field created-at|status-changed-at` to select the time filter and
`--sort requested-at|status-changed-at --order asc|desc` to control ordering.
`--status`, `--method`, and `--pg` accept repeated or comma-separated values.
`transactions` shows payment attempts and uses an experimental API.

### Structured output

```bash
portone payment view payment-xxx --json
portone payment list --json id,status
portone payment list --json --jq '.[] | .id'
```

`--json` prints the full API object, while `--json id,status` selects top-level
fields. `--jq/-q` requires `--json`. Lists emit arrays; view, cancellation, and
webhook resend emit objects. JSON retains the API's field names, statuses,
and integer amounts. An empty result is `[]` and exits successfully.

Without `--json`, terminal output uses readable tables and detail views.
Non-TTY lists are headerless TSV. Amounts are integer minor currency units.

### Cancellation and webhook results

`cancel` requires a reason and confirms the payment and requested cancellation
interactively. Pass `--yes` to skip confirmation; it is required when no TTY is
available. Omitting `--amount` cancels the full remaining amount.
`--tax-free-amount`, `--vat-amount`, and `--current-cancellable-amount` also
accept integer minor currency units.

For refund accounts and other complex fields, pass a complete JSON body with
`--input cancel.json` or `--input -`. It must contain a reason and cannot be
combined with individual cancellation field flags. A `storeId` in the body
overrides the default store, but must match an explicit `--store`.

A cancellation result of `REQUESTED` means the request was accepted and
`SUCCEEDED` means it completed; both exit with code 0. `FAILED` exits with code
1. Cancellations are not retried automatically.

Webhook resend executes without an additional confirmation. When
`--webhook-id` is omitted, the API selects the latest webhook. A successful
resend request and a successful delivery are distinct; a reported delivery
failure exits with code 1. Webhook request and response details are available
through `payment webhook list --json`.

## `portone api`

Make an authenticated PortOne V2 API request:

```bash
portone api <endpoint> [flags]
```

`<endpoint>` may be a path such as `/payments/{paymentId}` (replace the
placeholder with an actual value), a full URL, or `graphql`. The method defaults
to GET and switches to POST when request fields or `--input` supply a body.
Override it with `-X`. `--paginate` keeps REST requests on GET and GraphQL
requests on POST.

See the [command reference](https://github.com/portone-io/portone-cli/blob/main/docs/reference/portone_api.md)
for every flag. Only one of `--jq`, `--silent`, or `--verbose` may be used at a
time.

### Examples

Get one payment:

```bash
portone api /payments/{paymentId}
```

Send filters in the GET request body used by PortOne V2 list endpoints:

```bash
portone api /payments -X GET -F 'page[size]=10' -F 'filter[isTest]=true'
```

`key[sub]=value` creates a nested object and repeated `key[]=value` fields
create an array. `-F` converts integers, `true`, `false`, and `null` to JSON
types and supports `@file` or `@-` for file and standard-input values. `-f`
always sends a string.

Fetch every page and print payment IDs:

```bash
portone api /payments -X GET --paginate -q '.items[].id'
```

Use `--slurp` to wrap every page in one JSON array. It cannot be combined with
`--jq`:

```bash
portone api /payments -X GET --paginate --slurp
```

Read a request body from a file or standard input:

```bash
portone api /payments/{paymentId}/cancel --input cancel.json
echo '{"reason":"Customer request"}' | portone api /payments/{paymentId}/cancel --input -
```

Cache eligible GET, HEAD, and GraphQL responses for a TTL. Responses with a
403 or 5xx status are not cached. The default cache directory is
`~/.cache/portone`; override it with `PORTONE_CACHE_DIR`.

```bash
portone api /payments/{paymentId} --cache 1h
```

Pass `Authorization` explicitly with `-H` to override stored credentials. The
CLI also omits Authorization when a full endpoint URL has a different origin
from the configured base URL.

```bash
portone api /payments/{paymentId} -H 'Idempotency-Key: abc123'
portone api '/identity-verifications/{identityVerificationId}?storeId=store-xxx'
```

### GraphQL

Use `graphql` as the endpoint to request `{base URL}/graphql`. Every field other
than `query` and `operationName` is sent as a GraphQL variable:

```bash
portone api graphql -f query='query { merchant { ... on Merchant { id plainId } } }'

portone api graphql \
  -f query='query($id: ID!) { node(id: $id) { ... on Merchant { plainId } } }' \
  -f id='MDptZXJjaGFudC...'
```

A response containing an `errors` array exits with status 1 even when the HTTP
status is 200. The original JSON is written to stdout and the error message to
stderr.

GraphQL pagination requires an `$endCursor: String` variable and a
`pageInfo { hasNextPage endCursor }` selection. Use nested field syntax for
object variables:

```bash
portone api graphql --paginate --slurp \
  -f storeId='<store-global-id>' \
  -F 'filter[statuses][]=IN_PROGRESS' -F 'filter[cardCompanies][]' \
  -f query='
  query($storeId: ID!, $filter: PromotionFilterInput!, $endCursor: String) {
    node(id: $storeId) {
      ... on Store {
        promotions(filter: $filter, first: 50, after: $endCursor) {
          edges { node { id name status } }
          pageInfo { hasNextPage endCursor }
        }
      }
    }
  }'
```

### Embedded jq support

`-q` and `--jq` use the embedded [jaq](https://github.com/01mf02/jaq) engine, so
an external jq installation is not required. Most jq syntax and built-ins work,
but some built-ins may be unavailable or behave differently. For complex
transformations, piping the output to another tool remains an option.

### Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Success, including an output pipe that closes early |
| 1 | HTTP 4xx/5xx, GraphQL errors, invalid flag combinations, or other runtime errors |
| 2 | Command-line argument parsing error |

## `portone completion`

Generate completion scripts for Bash, Zsh, Fish, PowerShell, and Elvish:

```bash
# Zsh: save to a directory in $fpath
portone completion zsh > "${fpath[1]}/_portone"

# Bash
portone completion bash > "$(brew --prefix)/etc/bash_completion.d/portone"

# Fish
portone completion fish > ~/.config/fish/completions/portone.fish
```

Open a new shell to enable `portone <TAB>` completion.

## Command reference

Detailed documentation for every command and flag is available in
[docs/reference](https://github.com/portone-io/portone-cli/blob/main/docs/reference/index.md).
It is generated from the CLI definitions and checked by CI.

For patterns that help AI agents call `portone`, see
[skills/portone-cli/SKILL.md](https://github.com/portone-io/portone-cli/blob/main/skills/portone-cli/SKILL.md).
