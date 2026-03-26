# legend-cli

CLI binary and MCP server for [Legend](https://legend.xyz). Earn yield, trade RWAs, and more — without worrying about blockchains, bridges, or protocols.

This is the main entry point for the `legend-cli` workspace. For installation and getting started, see the [top-level README](../README.md).

## Usage

```bash
legend-cli login                      # authenticate via Google SSO
legend-cli accounts list              # list sub-accounts
legend-cli accounts create --keygen   # create account with a new P256 key
legend-cli folio <account_id>         # view portfolio

legend-cli plan earn <id> --amount 1000 --asset usdc --network base --protocol aave_v3 --execute
legend-cli plan swap <id> --sell-asset usdc --buy-asset weth --sell-amount 500 --network base --execute
legend-cli plan transfer <id> --amount 100 --asset usdc --network base --recipient 0x... --execute

legend-cli mcp serve                  # run as MCP server (stdio)
```

Pass `--execute` to create, sign, and submit a plan in one step.

## Configuration

Profiles are stored in `~/.legend/<env>/profiles/<name>.json`. The CLI supports multiple environments (`--dev`, `--stage`, `--prod`) and named profiles (`--profile <name>`).

## Global flags

| Flag | Description |
|---|---|
| `--profile <name>` | Use a named profile (default: `default`) |
| `--key <key>` | Override the query key |
| `--dev` / `--stage` / `--prod` | Target environment |
| `--json` | Force JSON output |
| `--quiet` | Minimal output (IDs only) |
| `-v` / `--verbose` | Log HTTP requests to stderr |

## License

MIT
