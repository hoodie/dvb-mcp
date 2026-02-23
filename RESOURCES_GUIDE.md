# MCP Resources Guide for DVB Server

This document outlines how to leverage the Model Context Protocol (MCP) Resources API in the DVB public transport server once support is available in the rmcp crate.

## What Are MCP Resources?

Resources in MCP expose contextual data that AI models can automatically access without explicit tool calls. They are:

- **URI-identified**: Each resource has a unique URI (e.g., `dvb://user/location`, `dvb://station/33000742`)
- **Subscribable**: Clients can subscribe to real-time updates
- **Auto-contextual**: AI can access them automatically without user prompting
- **Typed content**: Support text (JSON, Markdown) and binary data

## Benefits for DVB Server

### 1. Reduced Tool Calls
Instead of the AI calling `elicit_origin` every time, user context is pre-loaded as a resource.

### 2. Real-time Updates
Subscriptions allow pushing departure updates, trip progress, or delay notifications without polling.

### 3. Better Conversation Context
Multi-turn conversations maintain state through resources rather than repeated queries.

### 4. Automatic Context Inclusion
AI applications can automatically include relevant transit data in context.

## Proposed Resource Schema

### User Context Resources

#### `dvb://user/location`
**Purpose**: Current user location/origin
**Content Type**: `application/json`
**Annotations**:
- `audience`: `["user", "assistant"]`
- `priority`: `0.9` (high priority - nearly always needed)

```json
{
  "uri": "dvb://user/location",
  "name": "User Location",
  "description": "Current user location or origin point",
  "location": "Albertplatz",
  "station_id": "33000001",
  "coordinates": {
    "lat": 51.0634,
    "lon": 13.7526
  },
  "last_updated": "2024-01-15T14:30:00Z"
}
```

#### `dvb://user/destination`
**Purpose**: Current user destination
**Content Type**: `application/json`
**Annotations**:
- `audience`: `["user", "assistant"]`
- `priority`: `0.9`

```json
{
  "uri": "dvb://user/destination",
  "name": "User Destination",
  "description": "Current user destination point",
  "location": "Hauptbahnhof",
  "station_id": "33000028",
  "coordinates": {
    "lat": 51.0406,
    "lon": 13.7320
  },
  "last_updated": "2024-01-15T14:30:00Z"
}
```

#### `dvb://user/preferences`
**Purpose**: User travel preferences
**Content Type**: `application/json`
**Annotations**:
- `audience`: `["assistant"]`
- `priority`: `0.6`

```json
{
  "uri": "dvb://user/preferences",
  "name": "Travel Preferences",
  "favorite_stations": ["Albertplatz", "Hauptbahnhof", "Postplatz"],
  "preferred_transport": ["tram", "bus"],
  "avoid_transport": [],
  "accessibility_needs": false,
  "max_walking_minutes": 10
}
```

### Station Resources

#### `dvb://station/{id}`
**Purpose**: Detailed station information
**Content Type**: `application/json`
**Annotations**:
- `audience`: `["user", "assistant"]`
- `priority`: `0.7`

```json
{
  "uri": "dvb://station/33000001",
  "name": "Albertplatz",
  "station_id": "33000001",
  "coordinates": {
    "lat": 51.0634,
    "lon": 13.7526
  },
  "lines": [
    {"type": "tram", "number": "3", "direction": ["Wilder Mann", "Coschütz"]},
    {"type": "tram", "number": "6", "direction": ["Mockritz", "Niedersedlitz"]},
    {"type": "tram", "number": "11", "direction": ["Bühlau", "Zschertnitz"]}
  ],
  "facilities": ["elevator", "ticket_machine", "real_time_display"],
  "accessibility": true
}
```

#### `dvb://station/{id}/departures` (Subscribable)
**Purpose**: Real-time departure board
**Content Type**: `application/json`
**Subscribe**: `true`
**Annotations**:
- `audience`: `["user", "assistant"]`
- `priority`: `0.8`
- `lastModified`: Auto-updated on changes

```json
{
  "uri": "dvb://station/33000001/departures",
  "name": "Albertplatz Departures",
  "station_name": "Albertplatz",
  "station_id": "33000001",
  "last_updated": "2024-01-15T14:35:12Z",
  "departures": [
    {
      "line": "3",
      "direction": "Wilder Mann",
      "departure_time": "14:37",
      "departure_in_minutes": 2,
      "platform": "2",
      "delay_minutes": 0,
      "real_time": true
    },
    {
      "line": "11",
      "direction": "Bühlau",
      "departure_time": "14:40",
      "departure_in_minutes": 5,
      "platform": "1",
      "delay_minutes": 3,
      "real_time": true
    }
  ]
}
```

### Trip Resources

#### `dvb://trip/{trip_id}` (Subscribable)
**Purpose**: Real-time trip tracking
**Content Type**: `application/json`
**Subscribe**: `true`
**Annotations**:
- `audience`: `["user", "assistant"]`
- `priority`: `0.9` (when actively tracking)
- `lastModified`: Updated every 30 seconds

```json
{
  "uri": "dvb://trip/voe:11003::R:j24",
  "name": "Tram 3 to Wilder Mann",
  "trip_id": "voe:11003::R:j24",
  "line": "3",
  "direction": "Wilder Mann",
  "vehicle_type": "tram",
  "departure_time": "14:12",
  "current_delay": 0,
  "last_updated": "2024-01-15T14:35:00Z",
  "stops": [
    {
      "name": "Albertplatz",
      "scheduled": "14:12",
      "actual": "14:12",
      "status": "departed",
      "platform": "2"
    },
    {
      "name": "Carolaplatz",
      "scheduled": "14:15",
      "actual": "14:15",
      "status": "current",
      "platform": "1",
      "arrival_in": "30 seconds"
    },
    {
      "name": "Pirnaischer Platz",
      "scheduled": "14:17",
      "actual": "14:17",
      "status": "upcoming",
      "platform": "3"
    }
  ]
}
```

### Route Resources

#### `dvb://route/{route_id}`
**Purpose**: Saved route information
**Content Type**: `application/json`
**Annotations**:
- `audience`: `["user", "assistant"]`
- `priority`: `0.7`

```json
{
  "uri": "dvb://route/recent-1",
  "name": "Albertplatz → Hauptbahnhof",
  "route_id": "recent-1",
  "origin": "Albertplatz",
  "destination": "Hauptbahnhof",
  "connections": [
    {
      "line": "3",
      "departure": "14:12",
      "arrival": "14:20",
      "trip_id": "voe:11003::R:j24"
    }
  ],
  "duration_minutes": 8,
  "last_used": "2024-01-15T14:12:00Z"
}
```

### Resource Templates

#### `dvb://station/{station_id}/departures?limit={limit}&mot={mot}`
**Purpose**: Parameterized departure queries
**Parameters**:
- `station_id`: Station identifier (required)
- `limit`: Number of departures (optional, default: 10)
- `mot`: Mode of transport filter (optional, e.g., "tram,bus")

#### `dvb://trip/{trip_id}?time={time}&stop_id={stop_id}`
**Purpose**: Trip details at specific time/stop
**Parameters**:
- `trip_id`: Trip identifier (required)
- `time`: ISO 8601 timestamp (optional)
- `stop_id`: Specific stop ID (optional)

## Implementation Pattern (Future)

When rmcp adds resource support, implementation will likely follow this pattern:

```rust
use rmcp::{
    handler::server::resource::ResourceRouter,
    resource, resource_router,
};

#[resource_router]
impl DVBServer {
    /// User's current location resource
    #[resource(
        uri = "dvb://user/location",
        name = "User Location",
        description = "Current user location or origin point",
        mime_type = "application/json"
    )]
    async fn user_location(&self) -> Result<ResourceContents, McpError> {
        let location = self.user_location.lock().await;
        
        if let Some(loc) = location.as_ref() {
            Ok(ResourceContents::text(
                serde_json::to_string_pretty(&json!({
                    "location": loc,
                    "last_updated": Utc::now().to_rfc3339()
                }))?
            ))
        } else {
            Err(McpError::new(-32002, "Location not set"))
        }
    }
    
    /// Real-time departures resource (subscribable)
    #[resource(
        uri_template = "dvb://station/{id}/departures",
        name = "Station Departures",
        description = "Real-time departure board for a station",
        mime_type = "application/json",
        subscribe = true
    )]
    async fn station_departures(
        &self,
        id: String
    ) -> Result<ResourceContents, McpError> {
        // Fetch departures using dvb crate
        let departures = dvb::monitor(&id, 0, 10, dvb::Mot::All)
            .await
            .map_err(|e| McpError::new(-32603, e.to_string()))?;
        
        Ok(ResourceContents::text(
            serde_json::to_string_pretty(&departures)?
        ))
    }
}
```

## Subscription Use Cases

### 1. Active Trip Tracking
When user is tracking a trip, subscribe to `dvb://trip/{trip_id}` and push updates:
- Stop arrivals/departures
- Delay changes
- Platform changes

### 2. Departure Monitoring
Subscribe to `dvb://station/{id}/departures` for:
- Real-time updates while waiting
- Delay notifications
- Service disruptions

### 3. User Context Changes
Subscribe to `dvb://user/location` and `dvb://user/destination` to:
- React to location changes
- Suggest relevant routes
- Update favorite stations

## Integration with Current Tools

Resources complement (not replace) tools:

- **Tools**: Actions (search stations, plan routes, monitor departures)
- **Resources**: State and context (user location, recent trips, saved stations)

Example workflow:
1. AI reads `dvb://user/location` resource → knows user is at Albertplatz
2. User asks "When's my next tram?"
3. AI uses location from resource, calls `monitor_departures` tool
4. AI subscribes to `dvb://station/33000001/departures` for updates
5. AI presents departures and notifies on delays

## Migration Strategy

### Phase 1: Current State (Tools Only)
- ✅ All functionality via tools
- ✅ State stored in server memory
- ✅ Elicitation for user context

### Phase 2: Add Resources (When Available)
- Expose user location/destination as resources
- Add station information resources
- Implement basic subscriptions

### Phase 3: Enhanced Resources
- Add real-time departure resources
- Implement trip tracking resources
- Add route history resources

### Phase 4: Advanced Features
- Smart resource prioritization
- Predictive resource loading
- Context-aware resource suggestions

## Testing Resources

When implementing, test:
1. **List Resources**: Verify all resources appear
2. **Read Resources**: Check content format and accuracy
3. **Resource Templates**: Test parameter substitution
4. **Subscriptions**: Verify update notifications
5. **Error Handling**: Test invalid URIs, missing data
6. **Performance**: Ensure low latency for reads
7. **Concurrency**: Test multiple subscriptions

## Security Considerations

- **URI Validation**: Sanitize all resource URIs
- **Access Control**: Verify user permissions per resource
- **Rate Limiting**: Prevent subscription abuse
- **Data Privacy**: Don't expose sensitive location data
- **Sanitization**: Clean user input in resource parameters

## Monitoring & Observability

Track:
- Resource read latency
- Subscription count per resource
- Update notification frequency
- Cache hit/miss rates
- Resource errors

## Conclusion

Resources will significantly enhance the DVB MCP server by:
1. Reducing latency (fewer tool calls)
2. Improving context awareness (automatic state)
3. Enabling real-time updates (subscriptions)
4. Better user experience (proactive notifications)

**Next Steps**:
1. Monitor rmcp releases for resource support
2. Design resource URIs and schemas
3. Implement resource handlers
4. Add subscription logic
5. Update prompts to reference resources

---

**Status**: Waiting for rmcp v0.13+ with resource support
**Last Updated**: 2024-01-15
**Specification**: MCP 2025-11-25