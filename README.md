# Dresdner Verkehrsbetriebe MCP Server

A Model Context Protocol (MCP) server that empowers AI agents (and perhaps your coffee machine) to access Dresden’s public transit data, including trams, buses, and trains. This server makes it easy for systems to discover, plan, and monitor journeys using Dresden’s real-time and schedule-based transit information.

I’ve always found Dresden’s transit system fascinating, but navigating its data can be tricky. This MCP server aims to make route planning and schedule queries as seamless as possible for both humans and AI. [Here’s an example](https://claude.ai/share/ca2ac42a-b476-46f1-9b74-ef8ccd100ba0) of how an AI can use MCP to plan a tram journey across Dresden.
## Installation

### Prerequisites

- Rust (edition 2021 or higher)
- An MCP-compatible AI client (e.g. Claude Desktop, MCP Inspector)

### Building from Source

```bash
git clone https://github.com/YOUR_USERNAME/dvb-mcp.git
cd dvb-mcp

cargo build --release
```

## Configuration

The server can be configured using environment variables:

```bash
RUST_LOG=info
```

## MCP Prompts

This server provides the following MCP prompts for Dresden's transit system:

- `navigation-assistant`: Interactive assistant for comprehensive journey planning and navigation.
- `departure-monitor`: Real-time departure board for checking when the next vehicles are leaving from a specific station.
- `trip-tracker`: Track the progress of a trip and provide updates on its status.

## MCP Tools

This server provides the following MCP tools for Dresden's transit system:

- `elicit_origin`: Ask the user for their current location (starting point).
- `elicit_destination`: Ask the user for their desired destination.
- `find_stations`: Search for tram, bus, or train stations by name.
- `find_nearby_stations`: Find stations near a given location or landmark.
- `find_pois`: Search for points of interest in Dresden.
- `monitor_departures`: Get upcoming departures from a specified station.
- `list_lines`: List all lines departing from a station.
- `get_trip_details`: Get detailed information for a specific trip.
- `get_route_details`: Query possible routes between two stops.
- `lookup_stop_id_tool`: Look up the stop ID for a given station name.
- `osm_link`: Get an OpenStreetMap link for given coordinates.

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

1. Run: `npx @modelcontextprotocol/inspector`
2. Enter server command: `/path/to/dvb-mcp`

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

## Development

```bash
cargo build
cargo test
```

**Note:** The server communicates via stdin/stdout using the MCP protocol. Running it directly will cause it to wait for MCP protocol messages—this is normal! Use MCP Inspector or an MCP client to interact.

## Architecture

The server consists of several key components:

- **Transit Data Client**: Handles communication with Dresden’s transit APIs
- **MCP Tools**: Implements the MCP protocol tools for journey planning and schedule queries
- **Configuration**: Manages environment-based configuration

---

*Inspired by the [Dresden OpenData MCP Server](https://github.com/kiliankoe/dresden-opendata-mcp). May your journeys always be on time!*
