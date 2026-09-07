---
"@portone/cli": minor
---

Add PortOne V2 GraphQL API support through `portone api graphql`.

- Using `graphql` as the endpoint sends the request to `{base URL}/graphql`; every `-F` or `-f` field other than `query` and `operationName` is grouped under GraphQL variables.
- A response containing an `errors` array prints the message to stderr and exits with status 1 even when the HTTP status is 200.
- `--paginate` supports cursor pagination through an `$endCursor: String` variable and `pageInfo { hasNextPage endCursor }`; it remains a POST request and supports `--slurp`.
- `--cache` also caches POST responses from `graphql` requests.
