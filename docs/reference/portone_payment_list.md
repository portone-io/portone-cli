# portone payment list

List recent payments

```
portone payment list [OPTIONS]
```

## Options

| Option | Description |
| --- | --- |
| `-L, --limit <LIMIT>` | Maximum number of payments to fetch (1-60000) |
| `--status <STATUS>` | Filter by payment status (repeatable or comma-separated) [possible values: ready, pending, virtual-account-issued, paid, failed, partial-cancelled, cancelled] |
| `--method <METHOD>` | Filter by payment method (repeatable or comma-separated) [possible values: card, transfer, virtual-account, gift-certificate, mobile, easy-pay, convenience-store, crypto] |
| `--pg <PG>` | Filter by PG provider (repeatable or comma-separated) |
| `--currency <CURRENCY>` | Filter by currency code, such as KRW or USD |
| `--test` | Show only test payments |
| `--live` | Show only live payments |
| `--version <VERSION>` | Filter by PortOne payment version [possible values: v1, v2, all] |
| `--from <RFC3339>` | Start of the time range (default: 90 days before --until) |
| `--until <RFC3339>` | End of the time range (default: now) |
| `--time-field <TIME_FIELD>` | Timestamp used by --from and --until [possible values: created-at, status-changed-at] |
| `--sort <SORT>` | Field used to sort payments [possible values: requested-at, status-changed-at] |
| `--order <ORDER>` | Sort order [possible values: desc, asc] |
| `--search <TEXT>` | Search payment text |
| `--search-field <SEARCH_FIELD>` | Payment field to search |
| `--all-stores` | List all accessible stores, ignoring the default store |
| `--json [<FIELDS>]` | Output JSON, optionally selecting comma-separated fields |
| `-q, --jq <EXPR>` | Filter JSON output using a jq expression (requires --json) |
| `--profile <NAME>` | Configuration profile to use |
| `--base-url <URL>` | Base URL for API requests (default: https://api.portone.io) |
| `--store <STORE_ID>` | Store ID (default: PORTONE_STORE_ID or profile store_id) |

## See also

- [portone payment](portone_payment.md)
