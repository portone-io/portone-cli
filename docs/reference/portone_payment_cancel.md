# portone payment cancel

Cancel a payment

```
portone payment cancel [OPTIONS] <PAYMENT_ID>
```

## Arguments

| Argument | Description |
| --- | --- |
| `<PAYMENT_ID>` | Merchant-assigned payment ID |

## Options

| Option | Description |
| --- | --- |
| `--reason <REASON>` | Reason for cancelling the payment |
| `--amount <INTEGER>` | Amount to cancel in currency minor units (default: all remaining) |
| `--tax-free-amount <INTEGER>` | Tax-free cancellation amount in currency minor units |
| `--vat-amount <INTEGER>` | VAT cancellation amount in currency minor units |
| `--current-cancellable-amount <INTEGER>` | Expected cancellable balance in currency minor units |
| `--input <FILE>` | Read the cancellation JSON body from a file (use - for stdin) |
| `-y, --yes` | Skip confirmation (required when not running interactively) |
| `--json [<FIELDS>]` | Output JSON, optionally selecting comma-separated fields |
| `-q, --jq <EXPR>` | Filter JSON output using a jq expression (requires --json) |
| `--profile <NAME>` | Configuration profile to use |
| `--base-url <URL>` | Base URL for API requests (default: https://api.portone.io) |
| `--store <STORE_ID>` | Store ID (default: PORTONE_STORE_ID or profile store_id) |

## See also

- [portone payment](portone_payment.md)
