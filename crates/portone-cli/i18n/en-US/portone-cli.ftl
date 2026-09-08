core-config-read = failed to read config file: {$path}

core-config-invalid = invalid config file: {$path} ({$position})

core-config-directory = failed to create config directory: {$path}

core-config-serialize = failed to serialize config

core-config-save = failed to save config file: {$path}

core-response-parse = failed to parse response body: {$error}

core-jq-invalid = invalid jq filter: {$error}

core-jq-render = failed to render jq output: {$error}

core-output-binary = refusing to output binary content to the terminal; redirect or pipe stdout, or pass --allow-escape-sequences

core-output-escapes = the response contains terminal escape sequences; pass --allow-escape-sequences to output it anyway

core-field-invalid = invalid key: {$key}

core-field-value = field {$key} requires a value separated by an '=' sign

core-field-parse = error parsing {$key} value

core-field-array = expected array type under {$key}, got {$actual}

core-field-map = expected map type under {$key}, got {$actual}

core-field-override = unexpected override existing field under {$key}

core-field-open = open {$path}
core-header-value = header {$header} requires a value separated by ':'

core-header-name = invalid header name: {$name}

core-content-length = invalid Content-Length value: {$value}

core-input-stdin = failed to read request body from stdin

core-input-file = failed to read request body from {$path}

core-http-method = invalid HTTP method: {$method}

core-request-build = failed to build request

core-response-read = failed to read response body

core-request-log = Request to {$url}

core-body-omitted = body of {$bytes} bytes omitted

core-pagination-unknown = cannot determine pagination scheme; stopping after first page

core-pagination-limit = offset pagination limit (60000) reached; stopping

core-pagination-cursor = pagination cursor did not advance; stopping

core-pager-start = failed to start pager: {$error}

core-pager-invalid = invalid pager command

core-pager-empty = empty pager command

core-credentials-missing = no credentials found. run `portone auth login` or set PORTONE_ACCESS_TOKEN

core-paginate-method = the `--paginate` option is not supported for non-GET requests

core-paginate-input = the `--paginate` option is not supported with `--input`

core-slurp-jq = the `--slurp` option is not supported with `--jq`

core-slurp-paginate = `--paginate` required when passing `--slurp`

core-output-conflict = only one of `--jq`, `--silent`, or `--verbose` may be used

core-input-fields = the `--input` option is not supported with `--field` or `--raw-field`

help-about-portone = PortOne CLI

help-about-api = Make an authenticated PortOne V2 API request

help-about-auth = Authenticate with PortOne

help-about-login = Authenticate with PortOne Console

help-about-logout = Remove local credentials without revoking the server-side token

help-about-status = View authentication status

help-about-token = Print the current console access token

help-about-setup = Install PortOne plugins for AI coding assistants

help-about-completion = Generate shell completion scripts

help-about-help = Print this message or the help of the given subcommand(s)

help-about-payment = Inspect and manage payments
help-about-payment-list = List recent payments
help-about-payment-view = View a payment
help-about-payment-transactions = List payment attempts (unstable API)
help-about-payment-cancel = Cancel a payment
help-about-payment-webhook = Inspect and resend payment webhooks
help-about-payment-webhook-list = List webhooks for a payment
help-about-payment-webhook-resend = Resend a payment webhook
help-about-store = Configure the default store
help-about-store-set-default = Set the default store for a configuration profile
help-store-id = Store ID to save as the default
help-store-view = Print the saved default store ID
help-store-unset = Remove the saved default store
help-payment-store = Store ID (default: PORTONE_STORE_ID or profile store_id)
help-payment-id = Merchant-assigned payment ID
help-resource-json = Output JSON, optionally selecting comma-separated fields
help-resource-jq = Filter JSON output using a jq expression (requires --json)
help-payment-limit = Maximum number of payments to fetch (1-60000)
help-payment-status = Filter by payment status (repeatable or comma-separated)
help-payment-method = Filter by payment method (repeatable or comma-separated)
help-payment-pg = Filter by PG provider (repeatable or comma-separated)
help-payment-currency = Filter by currency code, such as KRW or USD
help-payment-test = Show only test payments
help-payment-live = Show only live payments
help-payment-version = Filter by PortOne payment version
help-payment-from = Start of the time range (default: 90 days before --until)
help-payment-until = End of the time range (default: now)
help-payment-time-field = Timestamp used by --from and --until
help-payment-sort = Field used to sort payments
help-payment-order = Sort order
help-payment-search = Search payment text
help-payment-search-field = Payment field to search
help-payment-all-stores = List all accessible stores, ignoring the default store
help-payment-cancel-reason = Reason for cancelling the payment
help-payment-cancel-amount = Amount to cancel in currency minor units (default: all remaining)
help-payment-cancel-tax-free = Tax-free cancellation amount in currency minor units
help-payment-cancel-vat = VAT cancellation amount in currency minor units
help-payment-cancel-current = Expected cancellable balance in currency minor units
help-payment-cancel-input = Read the cancellation JSON body from a file (use - for stdin)
help-payment-cancel-yes = Skip confirmation (required when not running interactively)
help-payment-webhook-id = Webhook to resend (default: the most recent webhook)

help-heading-commands = Commands

help-heading-arguments = Arguments

help-heading-options = Options

help-heading-usage = Usage:

help-flag-help = Print help

help-flag-help-short = Print help (see more with '--help')

help-flag-help-long = Print help (see a summary with '-h')

help-flag-version = Print version

help-profile-store = Configuration profile to store

help-profile-remove = Configuration profile to remove

help-profile-use = Configuration profile to use

help-base-url = Base URL for API requests (default: https://api.portone.io)

help-endpoint = Endpoint path, full URL, or graphql for the GraphQL API

help-method = HTTP method for the request (default: GET, or POST with fields)

help-fields = Add a typed request field in key=value format (supports @path, @-, integers, true, false, and null)

help-raw-fields = Add a string request field in key=value format

help-headers = Add an HTTP request header in key:value format

help-input = File to use as the request body (use "-" for standard input)

help-include = Include the response status line and headers in the output

help-paginate = Make additional requests to fetch all pages of results

help-slurp = Wrap all paginated JSON values in a single array (requires --paginate)

help-jq = Query the response using jq syntax

help-cache = Cache the response for a duration such as 3600s, 60m, or 1h

help-silent = Do not print the response body

help-verbose = Include the full HTTP request and response in the output

help-allow-escape-sequences = Allow printing terminal escape sequences

help-scopes = Comma-separated console scopes to request (default: HOME_AND_REPORT,TX_READ,CHANNEL_READ,STORE_READ,MERCHANT_READ)

help-insecure-storage = Store tokens in the config file instead of the OS keyring

help-no-browser = Print the login URL without opening a browser

help-show-secret = Display the access token without masking it

help-allow-dirty = Proceed even when the Git working tree is dirty

help-assistant = Assistant to configure (claude | codex | both)

help-shell = Shell for which to generate a completion script

help-subcommand = Print help for the subcommand(s)

help-api-long-about =
    Makes an authenticated HTTP request to the PortOne V2 API and prints the response.

    The `<ENDPOINT>` argument can be a REST path such as `/payments/{ "{" }paymentId{ "}" }`
    (replace placeholders with actual values), a full URL, or `graphql` for the
    GraphQL API. Paths are appended to `--base-url`, which defaults to
    `https://api.portone.io`. Full URLs are used as-is. The Authorization header is
    not sent when a full URL has a different origin from the base URL.

    The default HTTP method is `GET`, or `POST` when fields or `--input` provide a
    request body. Override the method with `--method`. PortOne V2 list endpoints
    accept filters in a GET request body, so pass `-X GET` when sending fields to
    one of these endpoints.

    Pass `-f/--raw-field` values in `key=value` format to add string fields.
    `-F/--field` performs type conversion based on the value:

    - `true`, `false`, `null`, and integer values are converted to JSON types;
    - values beginning with `@` are read from the remaining file path, and `@-`
      reads from standard input.

    Use `key[subkey]=value` for nested values, repeated `key[]=value` fields for
    arrays, and `key[]` without a value for an empty array.

    For GraphQL requests, every field other than `query` and `operationName` is
    sent as a GraphQL variable. A response with an `errors` array exits with status
    1 even when the HTTP status is 200.

    Pass a preconstructed body with `--input <FILE>`, or use `-` to read from
    standard input. `--input` cannot be combined with field flags or `--paginate`.

    With `--paginate`, requests continue until there are no more pages. REST
    pagination uses either `page.totalCount` for offset pagination or
    `items[].cursor` for cursor pagination. When no `page` field is supplied,
    offset pagination starts with `number=0, size=100`. GraphQL pagination requires
    an `$endCursor: String` variable and a `pageInfo { "{" } hasNextPage endCursor { "}" }`
    selection. Each page is printed as a separate JSON value; use `--slurp` to wrap
    all pages in one array.

    `-q/--jq` uses the embedded jaq engine and cannot be combined with `--slurp`.
    Only one of `--jq`, `--silent`, or `--verbose` may be used at a time.

    Environment variables:

    - `PORTONE_ACCESS_TOKEN`: console access token (takes precedence over profiles; not refreshed)
    - `PORTONE_API_BASE`: API base URL (`--base-url` > environment > profile > default)
    - `PORTONE_CONFIG_DIR`: configuration directory
    - `PORTONE_CACHE_DIR`: response cache directory used by `--cache`
    - `PORTONE_PAGER`, `PAGER`: pager for TTY output (`cat` or an empty value disables it)
    - `NO_COLOR`, `CLICOLOR_FORCE`: control color output

help-api-examples =
    # Get a payment (replace the placeholder with an actual ID)
    $ portone api /payments/{ "{" }paymentId{ "}" }

    # Add query parameters directly to the path
    $ portone api '/payments/{ "{" }paymentId{ "}" }?storeId=store-xxx'

    # List payments with filters in a GET request body
    $ portone api /payments -X GET -F 'page[size]=10' -F 'filter[isTest]=true'

    # Pass an array field
    $ portone api /payments -X GET \
      -F 'filter[methods][]=CARD' -F 'filter[methods][]=EASY_PAY'

    # Cancel a payment (fields switch the request to POST)
    $ portone api /payments/{ "{" }paymentId{ "}" }/cancel -f reason='Customer request'

    # Read a JSON request body from a file or standard input
    $ portone api /payments/{ "{" }paymentId{ "}" }/cancel --input cancel.json
    $ echo '{ "{" }"reason":"Customer request"{ "}" }' | portone api /payments/{ "{" }paymentId{ "}" }/cancel --input -

    # Add a custom header
    $ portone api /payments/{ "{" }paymentId{ "}" }/cancel -f reason=duplicate-payment \
      -H 'Idempotency-Key: abc123'

    # Fetch every page and print payment IDs
    $ portone api /payments -X GET --paginate -q '.items[].id'

    # Wrap all pages from a cursor-based endpoint in one JSON array
    $ portone api /payments-by-cursor -X GET --paginate --slurp

    # Cache a response for one hour
    $ portone api /payments/{ "{" }paymentId{ "}" } --cache 1h

    # Make a GraphQL query
    $ portone api graphql \
      -f query='query { "{" } merchant { "{" } ... on Merchant { "{" } id plainId { "}" } { "}" } { "}" }'

    # Pass GraphQL variables (all fields other than query become variables)
    $ portone api graphql -f id='<merchant-global-id>' -f query='
      query($id: ID!) { "{" } node(id: $id) { "{" } ... on Merchant { "{" } plainId { "}" } { "}" } { "}" }
    '

    # Paginate GraphQL results and build an object variable from nested fields
    $ portone api graphql --paginate --slurp \
      -f storeId='<store-global-id>' \
      -F 'filter[statuses][]=IN_PROGRESS' -F 'filter[cardCompanies][]' \
      -f query='
      query($storeId: ID!, $filter: PromotionFilterInput!, $endCursor: String) { "{" }
        node(id: $storeId) { "{" }
    { "      " }{ "..." } on Store { "{" }
            promotions(filter: $filter, first: 50, after: $endCursor) { "{" }
              edges { "{" } node { "{" } id name status { "}" } { "}" }
              pageInfo { "{" } hasNextPage endCursor { "}" }
    { "        " }{ "}" }
    { "      " }{ "}" }
    { "    " }{ "}" }
    { "  " }{ "}" }'

help-metadata-env = [env: { $value }]
help-metadata-default = [default: { $value }]
help-metadata-possible-values = [possible values: { $values }]

auth-invalid-redirect-uri = invalid redirect URI: { $uri }
auth-invalid-env-url = { $name } is not a URL: { $value }
auth-invalid-env-url-scheme = { $name } must be an HTTP or HTTPS URL: { $value }
auth-random-failed = failed to generate random bytes: { $error }
auth-token-request-failed = token request failed
auth-token-read-failed = failed to read token response
auth-token-parse-failed = failed to parse token response
auth-token-missing-access-token = token response is missing access_token
auth-token-missing-expires-in = token response is missing expires_in
auth-keyring-timeout = keyring did not respond within { $seconds } seconds
auth-stored-token-parse-failed = failed to parse stored tokens: { $error }
auth-refresh-lock-busy = another portone process is refreshing the token; try again shortly
auth-config-lock-busy = another portone process is updating the config file; try again shortly
auth-lock-directory-failed = failed to create lock directory: { $path }
auth-lock-open-failed = failed to open lock file: { $path }
auth-lock-acquire-failed = failed to acquire lock: { $path }
auth-browser-invalid-url = invalid URL { $url }: { $error }
auth-browser-unsupported-url = refusing to open non-http URL: { $url }
auth-keyring-load-failed = failed to read tokens for profile '{ $profile }' from the keyring ({ $service }/{ $id })
auth-session-expired = console login session has expired; run `portone auth login` to authenticate again
auth-refresh-rejected = token refresh was rejected ({ $error }): { $detail }
auth-refresh-failed-continuing = portone: token refresh failed; continuing with the current token: { $error }
auth-refresh-failed = token refresh failed
auth-refreshed-keyring-fallback = portone: failed to save refreshed tokens to the keyring; storing them in the config file: { $error }
auth-refreshed-save-failed = failed to save refreshed tokens; you may need to authenticate again on the next run
auth-validation-request-failed = console token validation request failed
auth-validation-parse-failed = failed to parse console token validation response
auth-callback-invalid-host = redirect URI host must be 127.0.0.1 or localhost: { $uri }
auth-callback-missing-port = redirect URI is missing a port: { $uri }
auth-callback-port-in-use = port { $port } is already in use; stop any other portone login or MCP server using it, then try again
auth-callback-start-failed = failed to start callback server on 127.0.0.1:{ $port }
auth-callback-timeout = timed out after waiting { $minutes } minutes for console login
auth-callback-accept-failed = failed to accept callback connection
auth-callback-invalid-request = Invalid request
auth-callback-method-not-allowed = Request not allowed
auth-callback-not-found = Not found
auth-callback-unverified-request = Unable to verify login request
auth-callback-state-mismatch-detail = The state value does not match. Restart login from the terminal.
auth-callback-state-mismatch = portone: ignored callback with mismatched state
auth-callback-denied-title = Login denied
auth-callback-denied-detail = Close this window and follow the instructions in your terminal.
auth-callback-complete-title = Login complete
auth-callback-complete-detail = Close this window and return to your terminal.
auth-callback-missing-code = The code value is missing.
auth-callback-request-line-too-long = request line too long
auth-callback-connection-closed = connection closed before request line
auth-login-env-active = portone: the { $name } environment variable is being used for authentication; unset it before storing login credentials
auth-login-browser-instructions = Complete console login in your browser. If the browser did not open, visit this URL:
auth-login-browser-failed = portone: failed to open browser: { $error }
auth-login-denied = console login was denied: { $error }{ $detail }
auth-login-token-failed = failed to obtain tokens
auth-login-missing-scopes = portone: some requested scopes were not granted: { $scopes }
auth-login-environment-mismatch = the issued token could not access { $base_url }; verify that the console and API environments match
auth-login-complete = Console login complete (merchant { $merchant })
auth-login-stored = Stored console login credentials in profile '{ $profile }'.
auth-source-keyring = keyring ({ $service }/{ $id })
auth-storage-file = config file (plain text)
auth-storage = Storage: { $location }
auth-login-keyring-timeout = the keyring did not respond within 30 seconds ({ $service }/{ $id }); check the keyring and try again
auth-login-keyring-fallback = portone: keyring is unavailable; storing tokens in the config file: { $error }
auth-login-cleanup-failed = portone: failed to delete previous console login tokens ({ $service }/{ $id }): { $error }
auth-logout-env-active = portone: the { $name } environment variable is being used for authentication; unset it before removing stored credentials
auth-profile-not-found = profile '{ $profile }' does not exist
auth-logout-keyring-delete-failed = failed to delete tokens from the keyring ({ $service }/{ $id })
auth-logout-removed = Removed profile '{ $profile }'.
auth-no-credentials = portone: no stored credentials found; run `portone auth login` to authenticate
auth-source-environment = environment variable { $name }
auth-source-config = config profile '{ $profile }'
auth-status-authentication = Authentication: Console OAuth
auth-status-source = Source: { $source }
auth-status-access-token = Access token: { $token }
auth-status-expires = Expires: { $timestamp } ({ $remaining })
auth-status-session-expires = Session expires: { $timestamp }
auth-status-scopes = Scopes: { $scopes }
auth-status-issued-by = Issued by: { $client_id } @ { $url }
auth-status-api-base-url = API base URL: { $url }
auth-status-valid = Validation: valid (merchant { $merchant })
auth-status-invalid = Validation: invalid
auth-status-invalid-token = portone: console token is invalid; run `portone auth login` to authenticate again
auth-remaining-expired = expired
auth-remaining-hours = { $hours }h { $minutes }m remaining
auth-remaining-minutes = { $minutes }m remaining
auth-remaining-seconds = { $seconds }s remaining

setup-starting = 🚀 Starting PortOne integration setup
setup-check-git = Checking Git status...
setup-git-dirty = The Git working tree has uncommitted changes
setup-allow-dirty-hint = Commit your changes or pass --allow-dirty to continue
setup-git-checked = Git status checked
setup-check-installation = Checking { $assistant } installation...
setup-not-installed = { $assistant } is not installed
setup-install-question = Install { $assistant }?
setup-installing = Installing { $assistant }...
setup-installed = Installed { $assistant }
setup-install-failed = Failed to install { $assistant }
setup-install-manually = Install { $assistant } manually: { $command }
setup-installation-found = { $assistant } installation found
setup-updating = Updating { $assistant }...
setup-updated = Updated { $assistant }
setup-update-failed = Failed to update { $assistant } (continuing)
setup-configuring-plugin = Configuring the PortOne plugin for { $assistant }...
setup-plugin-configured = Configured plugin for { $assistant }
setup-plugin-failed = Failed to configure plugin for { $assistant }
setup-complete = ✅ Setup complete!
setup-unsupported-assistant = Unsupported assistant: { $assistant }
setup-assistant-required = --assistant is required in non-interactive environments (claude | codex | both)
setup-assistant-question = Which assistant would you like to configure?
setup-selection-hint = ↑↓ to move, enter to select, type to filter
setup-prompt-canceled-indicator = <canceled>
setup-confirm-invalid-answer = Invalid answer, try typing 'y' for yes or 'n' for no
setup-confirm-yes = Yes
setup-confirm-no = No
setup-prompt-not-tty = The input device is not a TTY
setup-prompt-canceled = Operation was canceled by the user
setup-prompt-interrupted = Operation was interrupted by the user
setup-prompt-invalid-config = The prompt configuration is invalid: { $detail }
setup-prompt-io-error = IO error
setup-prompt-custom-error = User-provided error
setup-next-steps = 📋 Next steps
setup-start-assistant = 1. Start { $assistant }:
setup-run-slash-command = 2. Run this slash command:
setup-codex-prompts = 2. With the `portone-codex` plugin installed, try one of these prompts:
setup-example-implement = Implement a PortOne V2 one-time payment integration
setup-example-review = Review the PortOne integration in this project
setup-command-run-failed = failed to run command: { $command }
setup-command-output-failed = command failed: { $command }
    { $output }
setup-command-failed = command failed: { $command }
setup-create-directory-failed = failed to create directory: { $path }
setup-extract-assets-failed = failed to extract plugin assets: { $path }
setup-read-marketplace-failed = failed to read marketplace.json: { $path }
setup-write-marketplace-failed = failed to write marketplace.json: { $path }
setup-parse-marketplace-failed = failed to parse marketplace.json

resource-invalid-base-url = API base URL must be an HTTP or HTTPS URL
resource-invalid-path = Payment IDs cannot be . or .. or contain control characters.
resource-http-error = HTTP { $status }: { $detail }
resource-jq-requires-json = --jq requires --json
resource-json-field = unknown JSON field '{ $field }'; available fields: { $fields }
resource-status-requested = Requested
resource-status-succeeded = Succeeded
resource-status-failed = Failed
resource-status-ready = Ready
resource-status-pending = Pending
resource-status-virtual-account = Virtual account issued
resource-status-paid = Paid
resource-status-partial-cancelled = Partially cancelled
resource-status-cancelled = Cancelled
resource-no-results = No results found.
resource-label-id = ID
resource-label-status = Status
resource-label-amount = Amount (minor units)
resource-label-reason = Reason
resource-label-requested = Requested at
resource-label-cancelled-at = Cancelled at
resource-label-receipt = Receipt
resource-label-updated = Status changed at
resource-label-mode = Test/live
resource-label-order = Order
resource-label-pg-tx = PG transaction ID
resource-label-failure = Failure reason
resource-label-url = URL
resource-label-attempts = Attempts
resource-label-triggered = Triggered at
resource-label-store = Store
resource-label-version = Version
resource-label-method = Method
resource-label-channel = Channel
resource-label-transaction = Transaction ID
resource-label-paid = Paid at
resource-label-cancelled-amount = Cancelled amount (minor units)
resource-label-pg-code = PG code
resource-label-pg-message = PG message
resource-label-bank = Bank
resource-label-account = Account number
resource-label-account-holder = Account holder
resource-label-expires = Expires at
resource-label-cancellations = Cancellations
resource-label-webhooks = Webhooks

payment-empty-value = { $field } must not be empty
payment-response-field = API response is missing a valid { $field } field
payment-search-field-requires-search = --search-field requires --search
payment-list-all-stores = All accessible stores
payment-list-api-default = API default store scope
payment-list-scope = { $store } | { $version } | { $environment } | { $from } to { $until }
payment-date-range = the time range must be valid and --from must not be after --until
payment-date-invalid = { $field } must be an RFC3339 timestamp: { $value }
payment-cancel-needs-yes = --yes is required to cancel a payment in non-interactive environments
payment-cancel-failed = portone: the cancellation failed
payment-cancel-input-json = invalid cancellation JSON: { $error }
payment-cancel-input-amount = { $field } must be a valid integer in currency minor units (amount must be positive; other amounts may be zero)
payment-cancel-input-field = invalid or missing cancellation field: { $field }
payment-cancel-store-conflict = --store does not match storeId in the cancellation input
payment-cancel-all-remaining = All remaining
payment-cancel-preview =
    Profile: { $profile }
    Store: { $store }
    Payment: { $payment_id }
    Test/live: { $environment }
    Cancellation amount (minor units): { $amount } { $currency }
    Reason: { $reason }
payment-cancel-confirm = Cancel this payment? [y/N]
payment-cancel-aborted = Cancellation aborted.
payment-webhook-failed = portone: the webhook delivery failed

store-discovery-request-failed = failed to request accessible stores
store-discovery-parse-failed = failed to parse the store response
store-discovery-http = store discovery failed with HTTP { $status }
store-discovery-graphql = store discovery failed: { $detail }
store-discovery-union = store discovery returned { $kind }: { $detail }
store-discovery-malformed = store discovery returned an invalid response
store-selection-question = Select the default store
store-selection-hint = Use arrow keys to move, enter to select, or type to filter
store-selection-skip = Set up later
store-selection-representative = representative
store-selection-canceled = <canceled>
store-selection-failed = failed to select a default store
store-selection-empty = no accessible stores found
store-selection-requires-tty = a store ID is required in non-interactive environments
store-default-set = Default store set to { $store } for profile '{ $profile }'.
store-default-unset = Removed the default store from profile '{ $profile }'.
store-default-missing = profile '{ $profile }' has no default store
store-invalid-id = store ID must not be empty or contain control characters
auth-login-store-unavailable = portone: login succeeded, but the default store could not be determined: { $error }
auth-login-store-selected = Default store: { $store }
auth-login-store-unset = No default store selected. Run `portone store set-default` to set one later.
