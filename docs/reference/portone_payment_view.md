# portone payment view

View a payment

```
portone payment view [OPTIONS] <PAYMENT_ID>
```

## Arguments

| Argument | Description |
| --- | --- |
| `<PAYMENT_ID>` | Merchant-assigned payment ID |

## Options

| Option | Description |
| --- | --- |
| `--json [<FIELDS>]` | Output JSON, optionally selecting comma-separated fields |
| `-q, --jq <EXPR>` | Filter JSON output using a jq expression (requires --json) |
| `--profile <NAME>` | Configuration profile to use |
| `--base-url <URL>` | Base URL for API requests (default: https://api.portone.io) |
| `--store <STORE_ID>` | Store ID (default: PORTONE_STORE_ID or profile store_id) |

## See also

- [portone payment](portone_payment.md)
