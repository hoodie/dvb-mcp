# dvb-mcp CLI Usage

## Overview

The `dvb-mcp` server includes a CLI that automatically detects whether it's being run by an MCP client or directly in a terminal.

## Automatic Detection

When you run `dvb-mcp` without arguments:

- **In a terminal** (stdin is a TTY): Shows help and usage information
- **Via MCP client** (stdin is piped): Automatically starts in server mode

This behavior uses `std::io::IsTerminal` to detect the execution environment.

## Commands

### Server Mode

```bash
# Start the MCP server explicitly (optional - auto-detects when piped)
dvb-mcp serve
```

Forces the server to start in MCP mode, regardless of TTY detection. This command is **optional** - the server automatically detects when it's being run by an MCP client (when stdin is piped) and starts in server mode without needing this command.

### Introspection Commands

```bash
# List all available MCP tools
dvb-mcp list tools

# List all available MCP prompts
dvb-mcp list prompts

# List context keys (from UserContext schema)
dvb-mcp list context
```

These commands dynamically introspect the server's capabilities:
- `list tools`: Uses `ToolRouter::list_all()` to get tool metadata
- `list prompts`: Uses `PromptRouter::list_all()` to get prompt metadata
- `list context`: Uses JSON Schema reflection on the `UserContext` struct

### Version Information

```bash
# Show version
dvb-mcp --version
dvb-mcp -V
```

Version is automatically pulled from `Cargo.toml` using clap's built-in version attribute.

## Implementation Details

### TTY Detection

```rust
use std::io::IsTerminal;

if std::io::stdin().is_terminal() {
    // Show help - running in terminal
} else {
    // Start server - stdin is piped from MCP client
}
```

### No Hardcoded Lists

All introspection commands use runtime reflection:

- **Tools**: Extracted from `ToolRouter` via `list_all()`
- **Prompts**: Extracted from `PromptRouter` via `list_all()`
- **Context**: Extracted from `UserContext` JSON Schema via `schemars::schema_for!()`

This ensures the CLI always reflects the actual server capabilities without manual updates.

## MCP Client Integration

When an MCP client (like Claude Desktop or MCP Inspector) starts the server:

1. Client spawns `dvb-mcp` with stdin/stdout piped
2. TTY detection sees stdin is not a terminal
3. Server automatically enters MCP mode (no `serve` command needed)
4. Client sends `initialize` request
5. Server responds with capabilities
6. Communication proceeds via JSON-RPC over stdio

**No special arguments or configuration needed!** The `serve` command is completely optional and only useful for debugging or when auto-detection fails.
