# DVB MCP Server Architecture

This document describes the architecture of the Dresden public transport MCP server with full Resources, Tools, and Prompts support.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         MCP Client                               │
│                    (Claude Desktop, Cursor)                      │
└────────────┬────────────────────────────────────┬────────────────┘
             │                                    │
             │ MCP Protocol                       │ MCP Protocol
             │ (JSON-RPC 2.0)                    │ (JSON-RPC 2.0)
             │                                    │
┌────────────▼────────────────────────────────────▼────────────────┐
│                      DVB MCP Server                              │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              Server Capabilities                         │   │
│  │  • Resources  (automatic context access)                │   │
│  │  • Tools      (actions & queries)                       │   │
│  │  • Prompts    (conversation templates)                  │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │  Resources   │  │    Tools     │  │   Prompts    │          │
│  │   Handler    │  │   Router     │  │   Router     │          │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘          │
│         │                  │                  │                   │
│         │                  │                  │                   │
│  ┌──────▼──────────────────▼──────────────────▼───────────┐     │
│  │              Server State (In-Memory)                   │     │
│  │  • user_origin: Arc<Mutex<Option<String>>>             │     │
│  │  • user_location: Arc<Mutex<Option<String>>>           │     │
│  │  • user_destination: Arc<Mutex<Option<String>>>        │     │
│  └──────────────────────────────────────────────────────────┘    │
│                                                                   │
└────────────────────────────┬──────────────────────────────────────┘
                             │
                             │ dvb crate API
                             │
                ┌────────────▼────────────┐
                │   DVB Transit API       │
                │  (Dresden Transport)    │
                │                         │
                │  • Station Search       │
                │  • Route Planning       │
                │  • Departures           │
                │  • Trip Details         │
                └─────────────────────────┘
```

---

## Component Details

### 1. Resources (State & Context)

**Purpose**: Provide automatic access to server state without explicit tool calls

```
┌─────────────────────────────────────────────────────────────┐
│                      Resource Layer                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  dvb://user/context                                         │
│  ├─ Always available                                        │
│  ├─ Returns: origin, location, destination, status          │
│  └─ Used by: AI for automatic context awareness            │
│                                                              │
│  dvb://user/origin                                          │
│  ├─ Available when: origin is set                           │
│  ├─ Returns: journey starting point + timestamp             │
│  └─ Used by: AI for trip planning origin                    │
│                                                              │
│  dvb://user/location                                        │
│  ├─ Available when: location is set                         │
│  ├─ Returns: current location + timestamp                   │
│  └─ Used by: AI for "where user is right now"              │
│                                                              │
│  dvb://user/destination                                     │
│  ├─ Available when: destination is set                      │
│  ├─ Returns: destination + timestamp                        │
│  └─ Used by: AI for destination information                 │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Resource Lifecycle**:
1. Client calls `resources/list` → Server returns available resources
2. Client reads resource → Server returns current state from memory
3. State updated via tools → Resources reflect new state immediately
4. Resources always show live data (no caching)

### 2. Tools (Actions & Queries)

**Purpose**: Execute actions and query transit information

```
┌─────────────────────────────────────────────────────────────┐
│                        Tool Layer                            │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Context Management (Interactive)                           │
│  ├─ get_user_context      (get all context)                 │
│  ├─ elicit_origin         (ask user for origin)             │
│  ├─ elicit_location       (ask user for current location)   │
│  ├─ elicit_destination    (ask user for destination)        │
│  └─ reset_context         (clear all context)               │
│                                                              │
│  Context Management (Direct)                                │
│  ├─ set_origin            (set origin from conversation)    │
│  ├─ set_location          (set location from conversation)  │
│  └─ set_destination       (set dest from conversation)      │
│                                                              │
│  Station Operations                                         │
│  ├─ find_stations       (search by name)                    │
│  ├─ find_nearby_stations (proximity search)                 │
│  ├─ find_pois           (point of interest search)          │
│  └─ list_lines          (lines at station)                  │
│                                                              │
│  Real-time Information                                      │
│  ├─ monitor_departures  (departure board)                   │
│  └─ get_trip_details    (track specific trip)               │
│                                                              │
│  Journey Planning                                           │
│  ├─ get_route_details   (plan routes)                       │
│  └─ lookup_stop_id_tool (resolve station IDs)               │
│                                                              │
│  Utilities                                                  │
│  ├─ now                 (current time)                      │
│  └─ osm_link            (OpenStreetMap link)                │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Tool Execution Flow**:
1. Client calls tool with parameters
2. Tool validates input
3. Tool calls DVB API or accesses state
4. Tool returns result (success/error)
5. State updates propagate to resources

### 3. Prompts (Conversation Templates)

**Purpose**: Provide specialized conversation contexts for different use cases

```
┌─────────────────────────────────────────────────────────────┐
│                       Prompt Layer                           │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  navigation-assistant                                       │
│  ├─ Use case: General journey planning                      │
│  ├─ Behavior: Ask for origin/destination, plan routes       │
│  └─ Resources: Reads dvb://user/context automatically       │
│                                                              │
│  departure-monitor                                          │
│  ├─ Use case: Real-time departure boards                    │
│  ├─ Behavior: Quick answers for "next tram" queries         │
│  └─ Format: Markdown tables with line/time/platform         │
│                                                              │
│  trip-tracker                                               │
│  ├─ Use case: Track specific trips in real-time             │
│  ├─ Behavior: Monitor vehicle progress, delays, ETA         │
│  └─ Requires: trip_id from get_route_details                │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## Data Flow Examples

### Example 1: First Journey (Resources + Tools)

```
1. User asks: "How do I get to Hauptbahnhof?"

2. AI reads dvb://user/context resource
   └─> Returns: { status: "empty", origin: null, location: null, destination: null }

3. AI calls elicit_origin tool
   └─> User provides "Albertplatz"
   └─> Server updates state: user_origin = Some("Albertplatz")

4. AI recognizes destination from user's message
   └─> Calls set_destination("Hauptbahnhof")
   └─> Server updates state: user_destination = Some("Hauptbahnhof")
   └─> Server updates state: user_destination = Some("Hauptbahnhof")

5. AI calls get_route_details(origin: "Albertplatz", destination: "Hauptbahnhof")
   └─> DVB API returns route options
   └─> AI presents routes to user

6. Next query: "What time does it leave?"
   └─> AI reads dvb://user/context (now shows complete context!)
   └─> AI uses cached context without asking again
```

### Example 2: Real-time Departures (Prompts + Tools)

```
1. User selects "departure-monitor" prompt

2. Prompt context: "Focus on quick departure info, use tables"

3. User asks: "When's the next tram from Postplatz?"

4. AI calls find_stations("Postplatz")
   └─> Returns station info + ID

5. AI calls monitor_departures(stop_name: "Postplatz", limit: 10)
   └─> DVB API returns real-time departures
   
6. AI formats as Markdown table:
   | Line | Destination | Departure | Platform | Status |
   |------|-------------|-----------|----------|--------|
   | 1    | Prohlis     | 2 min     | 2        | On time|

7. User asks: "What about line 3?"
   └─> AI calls monitor_departures with mot filter
   └─> Shows only line 3 departures
```

### Example 3: Trip Tracking (All Components)

```
1. User used navigation-assistant to plan route
   └─> get_route_details returned trip_id: "voe:11003::R:j24"
   └─> AI stores trip_id in conversation context

2. User switches to "trip-tracker" prompt

3. User asks: "Where is my tram?"

4. AI reads dvb://user/context (has origin/location/destination)
   └─> Knows the journey context

5. AI calls get_trip_details(trip_id: "voe:11003::R:j24")
   └─> DVB API returns real-time stop sequence + progress
   
6. AI presents:
   ✓ Albertplatz (14:12) - Departed
   → Carolaplatz (14:15) - Arriving now
     Pirnaischer Platz (14:17)
     Hauptbahnhof (14:20)

7. User asks again 5 minutes later: "Update?"
   └─> AI calls get_trip_details again (same trip_id)
   └─> Shows updated progress
```

---

## State Management

### In-Memory State

```rust
pub struct DVBServer {
    // Session state (persists until server restart)
    user_origin: Arc<Mutex<Option<String>>>,      // Journey starting point
    user_location: Arc<Mutex<Option<String>>>,    // Current location
    user_destination: Arc<Mutex<Option<String>>>, // Destination
    
    // Routers (stateless)
    tool_router: ToolRouter<DVBServer>,
    prompt_router: PromptRouter<DVBServer>,
}
```

**Characteristics**:
- ✅ Thread-safe (Arc + Mutex)
- ✅ Session-scoped (cleared on restart)
- ✅ No persistence (privacy-friendly)
- ✅ Fast access (in-memory)

### State Transitions

```
┌─────────┐  set/elicit_origin  ┌──────────┐  set/elicit_destination  ┌──────────┐
│  Empty  │ ──────────────────> │ Partial  │ ──────────────────────>  │ Complete │
│         │                     │          │                          │          │
│ org: ✗  │                     │ org: ✓   │                          │ org: ✓   │
│ loc: ✗  │                     │ loc: ✗   │                          │ loc: ✓   │
│ dst: ✗  │                     │ dst: ✗   │                          │ dst: ✓   │
└─────────┘                     └──────────┘                          └──────────┘
     ▲                               │  ▲                                   │
     │                               │  │                                   │
     │         reset_context         │  │   set/elicit any field           │
     └───────────────────────────────┘  └───────────────────────────────────┘

Note: 
- Use set_* tools when user provides info directly: "I'm at X, I need to go to Y"
- Use elicit_* tools when you need to ask the user for missing info
```

---

## Resource vs Tool Decision Matrix

| Scenario | Use Resource | Use Tool | Reason |
|----------|-------------|----------|---------|
| Check user context | ✅ | ❌ | State query, no action |
| User says "I'm at X" | ❌ | ✅ (set_location) | Direct info from conversation |
| Need to ask where user is | ❌ | ✅ (elicit_location) | Interactive prompting |
| Plan route | ❌ | ✅ | Action with external API |
| Get departure times | ❌ | ✅ | Real-time query to API |
| Show conversation context | ✅ | ❌ | Automatic context loading |

**Rule of Thumb**:
- **Resources** = READ state (automatic, no side effects)
- **Tools** = WRITE state or QUERY external APIs (explicit actions)

---

## Performance Characteristics

### Resource Access
- **Latency**: ~10-50ms (in-memory read)
- **Caching**: Not needed (always fresh)
- **Concurrency**: Thread-safe via Mutex
- **Overhead**: Minimal

### Tool Execution
- **Latency**: 
  - State tools: ~50-100ms
  - DVB API tools: ~500-2000ms
- **Caching**: None (always real-time)
- **Rate Limiting**: Handled by DVB API
- **Retry**: Client-side

### Prompt Loading
- **Latency**: <10ms (in-memory)
- **Caching**: N/A (templates)
- **Overhead**: Negligible

---

## Error Handling

### Resource Errors
```
404 Resource Not Found
├─ dvb://user/origin when not set
├─ dvb://user/location when not set
├─ dvb://user/destination when not set
└─ Unknown URIs
```

### Tool Errors
```
Application Errors
├─ DVB API failures (network, timeout)
├─ Invalid parameters (validation)
├─ Empty search results
└─ Elicitation failures
```

### Error Response Format
```json
{
  "error": {
    "code": -32002,
    "message": "Resource not found",
    "data": {
      "uri": "dvb://user/location"
    }
  }
}
```

---

## Security Considerations

### Privacy
- ✅ No persistent storage of user data
- ✅ Session-only location tracking
- ✅ No external logging of locations
- ✅ State cleared on server restart

### Input Validation
- ✅ URI validation in read_resource
- ✅ Parameter validation in tools
- ✅ No SQL injection risk (no database)
- ✅ Rate limiting via DVB API

### Access Control
- ✅ Single-user session model
- ✅ No cross-session data leakage
- ✅ Resources scoped to session
- ✅ No authentication needed (local MCP)

---

## Future Architecture Extensions

### Phase 2: Real-time Resources
```
dvb://station/{id}/departures
├─ Resource Templates
├─ Parameter substitution
├─ Real-time updates
└─ Subscription support
```

### Phase 3: Resource Subscriptions
```
Client subscribes to dvb://station/123/departures
├─> Server sends notifications on updates
├─> Push model vs poll model
└─> Reduced latency for real-time data
```

### Phase 4: Caching Layer
```
┌─────────────┐
│ Resource    │ ──┐
│ Read        │   │
└─────────────┘   │
                  ▼
            ┌──────────┐
            │  Cache   │
            │  Layer   │
            └────┬─────┘
                 │
                 ▼
            ┌──────────┐
            │ DVB API  │
            └──────────┘
```

---

## Monitoring & Observability

### Key Metrics to Track
```
Resources:
├─ Read count per resource URI
├─ Read latency (p50, p95, p99)
├─ 404 rate per resource
└─ Concurrent reads

Tools:
├─ Call count per tool
├─ Success/error rate
├─ DVB API latency
└─ Parameter validation failures

Prompts:
├─ Prompt activation count
└─ Prompt execution time
```

### Logging Strategy
```
INFO: Resource reads
INFO: Tool executions
INFO: Prompt activations
WARN: DVB API failures
WARN: Invalid parameters
ERROR: Unexpected failures
```

---

## Deployment Architecture

```
┌──────────────────────────────────────────┐
│         MCP Client (Claude Desktop)       │
└────────────────┬─────────────────────────┘
                 │ stdio transport
                 │
┌────────────────▼─────────────────────────┐
│         DVB MCP Server Process            │
│                                           │
│  • Single process                         │
│  • Async runtime (tokio)                  │
│  • In-memory state                        │
│  • No persistence                         │
└────────────────┬─────────────────────────┘
                 │ HTTPS
                 │
┌────────────────▼─────────────────────────┐
│         DVB Transit API                   │
│         (dvbapi.de)                       │
└───────────────────────────────────────────┘
```

**Deployment Model**:
- Local process spawned by MCP client
- No server infrastructure needed
- State lives in process memory
- Restart clears all state

---

## Testing Strategy

### Unit Tests
- ✅ Tool parameter validation
- ✅ State transitions
- ✅ Resource URI parsing
- ✅ Error handling

### Integration Tests
- ✅ Resource read/write flow
- ✅ Tool execution with mock DVB API
- ✅ Prompt rendering
- ✅ End-to-end workflows

### Manual Tests
- ✅ Claude Desktop integration
- ✅ Multi-turn conversations
- ✅ Error scenarios
- ✅ Performance under load

---

## Dependencies

```toml
[dependencies]
rmcp = "0.12"           # MCP protocol implementation
dvb = "0.7.2"          # Dresden transit API client
tokio = "1.48"         # Async runtime
serde = "1.0"          # Serialization
serde_json = "1.0"     # JSON handling
chrono = "0.4"         # Date/time handling
anyhow = "1.0"         # Error handling
```

---

## Summary

The DVB MCP server implements a complete MCP architecture with:

✅ **Resources** - Automatic context access (3 resources)
✅ **Tools** - 13 transit operations + utilities
✅ **Prompts** - 3 specialized conversation templates
✅ **State Management** - Thread-safe in-memory session state
✅ **Performance** - Low latency, real-time updates
✅ **Privacy** - Session-only, no persistence
✅ **Extensibility** - Ready for subscriptions & templates

**Status**: Production-ready with rmcp v0.12.0

---

Last Updated: 2024-12-19
Version: 1.0.0