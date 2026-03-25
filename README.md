# legend-cli

CLI and MCP server for [Legend](https://legend.xyz). Earn yield, trade RWAs, and more — without worrying about blockchains, bridges, or protocols. Use it directly or let your agent handle it via [MCP](https://modelcontextprotocol.io).

Create an account right from your terminal, sync your signing keys with iCloud Keychain, and manage your portfolio across chains from one place.

## Install

### Homebrew (macOS)

```bash
brew install legend-hq/tap/legend-cli
```

### GitHub Releases

Pre-built binaries for macOS, Linux, and Windows are available on the [Releases](https://github.com/legend-hq/legend-cli/releases) page.

### From source

```bash
cargo install --path legend-cli
```

## Getting started

Log in with your Legend account:

```bash
legend-cli login
```

List accounts and view your portfolio:

```bash
legend-cli accounts list
legend-cli folio <account_id>
```

Earn yield in one step:

```bash
legend-cli plan earn <account_id> --amount 1000 --asset usdc --network base --protocol aave_v3 --execute
```

The `--execute` flag creates a plan, signs it with your local key, and submits it automatically.

## MCP

Legend supports [Claude Code](https://docs.anthropic.com/en/docs/claude-code), [OpenClaw](https://openclaw.ai), and any MCP-compatible client.

### Claude Code

```bash
claude mcp add legend -- legend-cli mcp serve
```

### Claude Desktop / other MCP clients

```json
{
  "mcpServers": {
    "legend": {
      "command": "legend-cli",
      "args": ["mcp", "serve"]
    }
  }
}
```

## Workspace

This repo is a Cargo workspace with three crates:

- **legend-cli** — CLI binary and MCP server
- **legend-client** — Rust client library for the Legend API
- **legend-signer** — P256 signing via macOS Keychain, file keys, or Turnkey

## Authentication

Credentials resolve in order: `--key` flag, `LEGEND_QUERY_KEY` env var, then saved profile. Use `--profile <name>` to manage multiple identities.

## License

MIT
