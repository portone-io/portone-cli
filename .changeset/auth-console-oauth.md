---
"@portone/cli": minor
---

Use PortOne Console login (OAuth authorization code with PKCE) for CLI authentication. `portone auth login` now starts browser login directly.

- Store browser-issued console tokens in the OS keyring, fall back to the config file when unavailable, and support explicit file storage with `--insecure-storage`. `portone api`, `auth status`, and `auth token` refresh tokens 60 seconds before expiry, with refreshes serialized by an interprocess lock.
- Use `Authorization: Bearer` for both REST and GraphQL requests, and retain the issuing console URL, token endpoint, and API base URL in each profile.
- Add `portone auth token` to print the current console access token for tools such as the MCP server (`PORTONE_ACCESS_TOKEN=$(portone auth token)`).
- Accept an externally issued console token through `PORTONE_ACCESS_TOKEN`.
- Expand `auth status` with the authentication method, access and session expiry, scopes, and issuing environment. Authorization masking in `--verbose` output now preserves the scheme.
- Reject `auth login` and `auth logout` while `PORTONE_ACCESS_TOKEN` is set, and point missing-credential errors to that environment variable.
