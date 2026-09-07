# portone auth login

Authenticate with PortOne Console

```
portone auth login [OPTIONS]
```

## Options

| Option | Description |
| --- | --- |
| `--profile <NAME>` | Configuration profile to store |
| `--base-url <URL>` | Base URL for API requests (default: https://api.portone.io) |
| `--scopes <SCOPES>` | Comma-separated console scopes to request (default: HOME_AND_REPORT,TX_READ,CHANNEL_READ,STORE_READ,MERCHANT_READ) |
| `--insecure-storage` | Store tokens in the config file instead of the OS keyring |
| `--no-browser` | Print the login URL without opening a browser |

## See also

- [portone auth](portone_auth.md)
