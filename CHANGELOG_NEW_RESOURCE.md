# New Resource: dvb://departures/{stop_id}

## Summary

Added a new MCP resource template `dvb://departures/{stop_id}` that provides real-time departure information for a specific stop via the MCP resources API.

## What Changed

### New Resource Template

**URI Template**: `dvb://departures/{stop_id}`

**Description**: Real-time departure information for a specific stop. Use the stop_id from find_stations or lookup_stop_id.

**Content Type**: `application/json`

**Example Usage**:
- URI: `dvb://departures/33000001` (for Albertplatz)
- Returns: JSON with stop_id, departures array, and last_updated timestamp

### Implementation Details

1. **Resource Template Registration** (`list_resource_templates`)
   - Added `RawResourceTemplate` with:
     - `uri_template`: "dvb://departures/{stop_id}"
     - `name`: "Station Departures"
     - `title`: "Real-time Departures"
     - `description`: Usage instructions
     - `mime_type`: "application/json"

2. **Resource Reading** (`read_resource`)
   - Added URI pattern matching for "dvb://departures/" prefix
   - Extracts `stop_id` from URI path
   - Calls `dvb::monitor::departure_monitor()` with default parameters:
     - `mot`: None (all transport types)
     - `limit`: 10 departures
   - Returns JSON response with:
     - `stop_id`: The requested stop identifier
     - `departures`: Array of departure objects from dvb crate
     - `last_updated`: ISO 8601 timestamp
   - Proper error handling with `McpError::resource_not_found`

### Response Format

```json
{
  "stop_id": "33000001",
  "departures": [
    {
      "line": "3",
      "direction": "Wilder Mann",
      "departure_time": "15:37",
      "platform": "2",
      "mode": "Tram"
      // ... other fields from dvb crate
    }
  ],
  "last_updated": "2024-12-19T15:35:00+01:00"
}
```

## Benefits

1. **Direct Access**: AI assistants can read departure information directly via resource URI
2. **No Tool Call**: Reduces latency compared to `monitor_departures` tool
3. **Bookmarkable**: Resource URIs can be referenced/saved for frequent stops
4. **Discoverable**: Templates listed via `list_resource_templates` endpoint
5. **Consistent**: Follows same pattern as other MCP resources

## Files Modified

1. **src/server.rs**
   - `read_resource()`: Added URI pattern matching and departure fetching logic
   - `list_resource_templates()`: Added departures template registration

2. **RESOURCES_IMPLEMENTED.md**
   - Documented the new resource template
   - Updated implementation details and line references
   - Added to future enhancements tracking

3. **README.md**
   - Added MCP Resources section
   - Documented the departures resource template
   - Added usage examples

4. **CHANGELOG_NEW_RESOURCE.md** (this file)
   - Comprehensive documentation of changes

## Testing

Build successful:
```bash
$ cargo build
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.31s
```

Check successful:
```bash
$ cargo check
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.25s
```

## Usage Examples

### With MCP Inspector

```bash
# List resource templates
→ resources/listTemplates

# Read departures for Albertplatz
→ resources/read { "uri": "dvb://departures/33000001" }
```

### With Claude Desktop

```
User: "Show me departures for stop 33000001"
Claude: [Reads dvb://departures/33000001 resource]
Claude: "Here are the upcoming departures from this stop: ..."
```

## Integration Notes

- Fully compatible with existing tools and resources
- Backward compatible (existing functionality unchanged)
- No breaking changes
- Works with rmcp 0.12.0

## Next Steps

Potential future enhancements:
1. Add query parameters: `dvb://departures/{stop_id}?limit={n}&mot={modes}`
2. Enable resource subscriptions for real-time updates
3. Add `dvb://trip/{trip_id}` resource template for trip tracking
4. Add `dvb://route/{route_id}` for saved routes

## References

- MCP Specification: https://spec.modelcontextprotocol.io/
- rmcp crate: https://crates.io/crates/rmcp
- dvb crate: https://crates.io/crates/dvb

---

**Date**: 2024-12-19
**Version**: 0.1.0
**Status**: ✅ Complete
