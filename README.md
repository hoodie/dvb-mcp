# Dresdner Verkehrsbetriebe MCP Server

A Model Context Protocol (MCP) server that empowers AI agents (and perhaps your coffee machine) to access Dresden’s public transit data, including trams, buses, and trains. This server makes it easy for systems to discover, plan, and monitor journeys using Dresden’s real-time and schedule-based transit information.

I’ve always found Dresden’s transit system fascinating, but navigating its data can be tricky. This MCP server aims to make route planning and schedule queries as seamless as possible for both humans and AI. [Here’s an example](https://claude.ai/share/ca2ac42a-b476-46f1-9b74-ef8ccd100ba0) of how an AI can use MCP to plan a tram journey across Dresden.

## Installation

```bash
cargo install dvb-mcp
```

### Prerequisites

- Rust (edition 2024 or higher)
- An MCP-compatible AI client (e.g. Claude Desktop, MCP Inspector)

### Building from Source

```bash
git clone https://github.com/YOUR_USERNAME/dvb-mcp.git
cd dvb-mcp

cargo build --release
```

## Usage

### Running the Server

The server automatically detects how it's being run:

- **Via MCP Client** (stdin piped): Automatically starts in server mode
- **In Terminal** (interactive): Shows help and usage information
- **Explicit Server Mode**: Use `dvb-mcp serve` to force server mode (optional)

```bash
# Show help and available commands
dvb-mcp

# Start server explicitly (optional - auto-detects when piped)
dvb-mcp serve

# List available tools
dvb-mcp list tools

# List available prompts
dvb-mcp list prompts

# List context keys
dvb-mcp list context

# Show version
dvb-mcp --version
dvb-mcp -V
```

### Configuration

The server can be configured using environment variables:

```bash
RUST_LOG=info
```

## MCP Resources

This server provides MCP resources for automatic context access:

### Static Resources
- `dvb://user/context`: Complete user context (origin, location, destination)
- `dvb://user/location`: Current user location (when set)
- `dvb://user/destination`: User destination (when set)

### Resource Templates
- `dvb://departures/{stop_id}`: Real-time departure information for a specific stop

**Benefits**: Resources are automatically available to AI assistants without requiring explicit tool calls, providing faster context access and more natural conversations.

See `RESOURCES_IMPLEMENTED.md` for detailed documentation.

## MCP Prompts

This server provides the following MCP prompts for Dresden's transit system:

- `navigation-assistant`: Interactive assistant for comprehensive journey planning and navigation.
- `departure-monitor`: Real-time departure board for checking when the next vehicles are leaving from a specific station.
- `trip-tracker`: Track the progress of a trip and provide updates on its status.

## MCP Tools

This server provides the following MCP tools for Dresden's transit system:

### Context Management

**Interactive Elicitation** (for prompting user):
- `elicit_origin`: Ask the user for their journey origin/starting point.
- `elicit_location`: Ask the user for their current location.
- `elicit_destination`: Ask the user for their desired destination.

**Direct Setting** (when user provides info in conversation):
- `set_origin`: Set the journey starting point when user says "I'm starting from X".
- `set_location`: Set current location when user says "I'm at X".
- `set_destination`: Set destination when user says "I need to go to X".

**Context Retrieval**:
- `get_user_context`: Get all saved context (origin, location, destination) in one call.
- `reset_context`: Clear all saved context.

### Transit Operations

- `find_stations`: Search for tram, bus, or train stations by name.
- `find_nearby_stations`: Find stations near a given location or landmark.
- `find_pois`: Search for points of interest in Dresden.
- `monitor_departures`: Get upcoming departures from a specified station.
- `list_lines`: List all lines departing from a station.
- `get_trip_details`: Get detailed information for a specific trip.
- `get_route_details`: Query possible routes between two stops.
- `lookup_stop_id_tool`: Look up the stop ID for a given station name.
- `osm_link`: Get an OpenStreetMap link for given coordinates.
- `now`: Get the current local time in ISO8601 format.

### User Context Concepts

The server maintains three distinct user context fields:

- **Origin**: Where the user's journey *starts* (set at trip planning time)
- **Location**: Where the user *currently is* (can change during journey)
- **Destination**: Where the user *wants to go*

**Example Usage**:
1. User: "I'm at Hauptbahnhof and need to get to Altmarkt"
   - Agent calls: `set_location("Hauptbahnhof")` and `set_destination("Altmarkt")`
2. User: "Where am I starting from?"
   - Agent calls: `elicit_origin()` to ask interactively

## Usage with Claude Desktop

Add the server to your Claude Desktop configuration:

```json
{
  "mcpServers": {
    "dvb-mcp": {
      "command": "/path/to/dvb-mcp"
    }
  }
}
```

## Usage with MCP Inspector

To test with MCP Inspector:

```bash
npx @modelcontextprotocol/inspector /path/to/dvb-mcp
```

The server will automatically detect it's being run by an MCP client and start in server mode. The `serve` command is not needed - it's only useful for forcing server mode when auto-detection fails.

## Example Usage with Claude

[Here's an example](https://claude.ai/share/41f6ee24-1f5d-4e54-9d34-1645ad55b457) interaction with Claude using this MCP server to find a route and schedule for Dresden's trams.

### Using the Departure Monitor

Ask Claude to use the `departure-monitor` prompt for quick departure information:

```
"When is the next tram from Postplatz?"
"Show me departures from Hauptbahnhof"
"What's leaving from Albertplatz right now?"
```

The departure monitor provides fast, focused answers about upcoming departures without requiring full journey planning.

### Using Resources

The server automatically provides context through MCP resources. For example:

```
"How do I get to Hauptbahnhof?"
```

Claude will automatically read your saved location from `dvb://user/context` and plan the route without asking where you are.

To access real-time departures via resources:

```
"Show me departures for stop 33000001"
```

Claude can read the `dvb://departures/33000001` resource directly for instant departure information.

## Development

```bash
cargo build
cargo test
```

**Note:** The server uses TTY detection to determine if it's being run by an MCP client or directly in a terminal:
- When run directly in a terminal, it displays help information
- When started by an MCP client (stdin piped), it automatically enters server mode
- The `serve` command is optional and only needed to force server mode if auto-detection fails

## Architecture

The server consists of several key components:

- **Transit Data Client**: Handles communication with Dresden’s transit APIs
- **MCP Tools**: Implements the MCP protocol tools for journey planning and schedule queries
- **Configuration**: Manages environment-based configuration

---

*Inspired by the [Dresden OpenData MCP Server](https://github.com/kiliankoe/dresden-opendata-mcp). May your journeys always be on time!*
