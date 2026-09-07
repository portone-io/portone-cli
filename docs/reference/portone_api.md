# portone api

Makes an authenticated HTTP request to the PortOne V2 API and prints the response.

The `<ENDPOINT>` argument can be a REST path such as `/payments/{paymentId}`
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
an `$endCursor: String` variable and a `pageInfo { hasNextPage endCursor }`
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

```
portone api [OPTIONS] <ENDPOINT>
```

## Arguments

| Argument | Description |
| --- | --- |
| `<ENDPOINT>` | Endpoint path, full URL, or graphql for the GraphQL API |

## Options

| Option | Description |
| --- | --- |
| `-X, --method <METHOD>` | HTTP method for the request (default: GET, or POST with fields) |
| `-F, --field <key=value>` | Add a typed request field in key=value format (supports @path, @-, integers, true, false, and null) |
| `-f, --raw-field <key=value>` | Add a string request field in key=value format |
| `-H, --header <key:value>` | Add an HTTP request header in key:value format |
| `--input <FILE>` | File to use as the request body (use "-" for standard input) |
| `-i, --include` | Include the response status line and headers in the output |
| `--paginate` | Make additional requests to fetch all pages of results |
| `--slurp` | Wrap all paginated JSON values in a single array (requires --paginate) |
| `-q, --jq <EXPR>` | Query the response using jq syntax |
| `--cache <TTL>` | Cache the response for a duration such as 3600s, 60m, or 1h |
| `--silent` | Do not print the response body |
| `--verbose` | Include the full HTTP request and response in the output |
| `--allow-escape-sequences` | Allow printing terminal escape sequences |
| `--base-url <URL>` | Base URL for API requests (default: https://api.portone.io) |
| `--profile <NAME>` | Configuration profile to use |

## Examples

```sh
# Get a payment (replace the placeholder with an actual ID)
$ portone api /payments/{paymentId}

# Add query parameters directly to the path
$ portone api '/payments/{paymentId}?storeId=store-xxx'

# List payments with filters in a GET request body
$ portone api /payments -X GET -F 'page[size]=10' -F 'filter[isTest]=true'

# Pass an array field
$ portone api /payments -X GET \
  -F 'filter[methods][]=CARD' -F 'filter[methods][]=EASY_PAY'

# Cancel a payment (fields switch the request to POST)
$ portone api /payments/{paymentId}/cancel -f reason='Customer request'

# Read a JSON request body from a file or standard input
$ portone api /payments/{paymentId}/cancel --input cancel.json
$ echo '{"reason":"Customer request"}' | portone api /payments/{paymentId}/cancel --input -

# Add a custom header
$ portone api /payments/{paymentId}/cancel -f reason=duplicate-payment \
  -H 'Idempotency-Key: abc123'

# Fetch every page and print payment IDs
$ portone api /payments -X GET --paginate -q '.items[].id'

# Wrap all pages from a cursor-based endpoint in one JSON array
$ portone api /payments-by-cursor -X GET --paginate --slurp

# Cache a response for one hour
$ portone api /payments/{paymentId} --cache 1h

# Make a GraphQL query
$ portone api graphql \
  -f query='query { merchant { ... on Merchant { id plainId } } }'

# Pass GraphQL variables (all fields other than query become variables)
$ portone api graphql -f id='<merchant-global-id>' -f query='
  query($id: ID!) { node(id: $id) { ... on Merchant { plainId } } }
'

# Paginate GraphQL results and build an object variable from nested fields
$ portone api graphql --paginate --slurp \
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

## See also

- [portone](portone.md)
