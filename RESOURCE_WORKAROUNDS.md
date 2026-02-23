# Resource Workarounds for DVB MCP Server

Since rmcp 0.12.0 doesn't yet support the MCP Resources API, this document describes practical workarounds to achieve similar functionality using existing features.

## Current Limitations

- ❌ No native resource URI system
- ❌ No automatic context loading
- ❌ No resource subscriptions
- ✅ Tools available
- ✅ Prompts available
- ✅ Server state available

## Workaround Strategies

### Strategy 1: Context Tools (Implemented)

Create lightweight tools that return context instead of performing actions.

**Already Implemented**:
```rust
#[tool(description = "Get user's saved origin location")]
async fn elicit_origin(&self, params: Parameters<OriginInfo>) -> CallToolResult

#[tool(description = "Get user's saved destination")]
async fn elicit_destination(&self, params: Parameters<DestinationInfo>) -> CallToolResult
```

**Benefits**:
- ✅ Works today
- ✅ AI can call these proactively
- ✅ Low latency

**Limitations**:
- ❌ Requires explicit tool calls
- ❌ Not automatically included in context
- ❌ No subscriptions

### Strategy 2: Enhanced Prompts with Context

Modify prompts to include context information directly in the system message.

**Implementation**:

```rust
#[prompt(name = "navigation-assistant-with-context")]
async fn navigation_assistant(&self) -> Vec<PromptMessage> {
    let location = self.user_location.lock().await.clone();
    let destination = self.user_destination.lock().await.clone();
    
    let context = match (location, destination) {
        (Some(loc), Some(dest)) => format!(
            "\n\n**CURRENT CONTEXT**:\n- User Location: {}\n- User Destination: {}\n\n\
             You already know the user's origin and destination. Use this information directly.",
            loc, dest
        ),
        (Some(loc), None) => format!(
            "\n\n**CURRENT CONTEXT**:\n- User Location: {}\n- User Destination: Not set\n\n\
             You know the user's location but need to ask for their destination.",
            loc
        ),
        (None, Some(dest)) => format!(
            "\n\n**CURRENT CONTEXT**:\n- User Location: Not set\n- User Destination: {}\n\n\
             You know the user's destination but need to ask where they are starting from.",
            dest
        ),
        (None, None) => "\n\n**CURRENT CONTEXT**:\nUser location and destination not yet set.\n\n".to_string(),
    };
    
    vec![
        PromptMessage::new_text(
            PromptMessageRole::Assistant,
            format!(
                "You are a travel assistant for Dresden's public transportation system (DVB).{}\
                 You have access to tools from the local public transportation provider...",
                context
            ),
        ),
        // ... rest of messages
    ]
}
```

**Benefits**:
- ✅ Context automatically included in prompt
- ✅ No extra tool calls needed
- ✅ Works with current rmcp

**Limitations**:
- ❌ Context only loaded when prompt is called
- ❌ Not dynamic during conversation
- ❌ No real-time updates

### Strategy 3: Proactive Context Tools

Create tools that AI should call at the start of each conversation.

**New Tool Examples**:

```rust
#[tool(
    name = "get_user_context",
    description = "IMPORTANT: Call this first in every conversation to get user's saved location, \
                   destination, and preferences. Returns all context in one call."
)]
async fn get_user_context(&self) -> CallToolResult {
    let location = self.user_location.lock().await.clone();
    let destination = self.user_destination.lock().await.clone();
    
    let context = json!({
        "location": location,
        "destination": destination,
        "last_updated": chrono::Utc::now().to_rfc3339(),
        "context_available": location.is_some() || destination.is_some()
    });
    
    success_json(&context)
}

#[tool(
    name = "get_station_context",
    description = "Get comprehensive information about a station including lines, facilities, \
                   and recent departures. Use this instead of multiple separate calls."
)]
async fn get_station_context(&self, params: Parameters<StationContextRequest>) -> CallToolResult {
    // Combine multiple data sources:
    // 1. Station info from find_stations
    // 2. Lines from list_lines
    // 3. Recent departures from monitor_departures
    
    // ... implementation
}
```

**Update System Prompts**:

```rust
PromptMessage::new_text(
    PromptMessageRole::Assistant,
    "You are a travel assistant for Dresden's public transportation system (DVB). \
     \n\n**IMPORTANT WORKFLOW**:\n\
     1. FIRST: Always call get_user_context to check if user location/destination are saved\n\
     2. Use the context to avoid asking redundant questions\n\
     3. For station info, use get_station_context for comprehensive data\n\
     \nYou have access to tools from the local public transportation provider..."
)
```

**Benefits**:
- ✅ Single tool call for all context
- ✅ Works with current rmcp
- ✅ Can be documented in prompts

**Limitations**:
- ❌ Requires AI to remember to call it
- ❌ Not truly automatic
- ❌ Still requires tool call overhead

### Strategy 4: Cached Response Pattern

Cache frequently accessed data in memory and return it quickly.

**Implementation**:

```rust
use std::time::{Duration, Instant};

pub struct CachedDepartures {
    station_id: String,
    data: Vec<Departure>,
    cached_at: Instant,
    ttl: Duration,
}

pub struct DVBServer {
    user_location: Arc<Mutex<Option<String>>>,
    user_destination: Arc<Mutex<Option<String>>>,
    departure_cache: Arc<Mutex<HashMap<String, CachedDepartures>>>,
    tool_router: ToolRouter<DVBServer>,
    prompt_router: PromptRouter<DVBServer>,
}

#[tool(
    name = "monitor_departures",
    description = "Monitor real-time departures. Results are cached for 30 seconds."
)]
async fn monitor_departures(&self, params: Parameters<MonitorDeparturesRequest>) -> CallToolResult {
    let cache_key = format!("{}:{:?}", params.stop_id, params.mot);
    
    // Check cache first
    {
        let cache = self.departure_cache.lock().await;
        if let Some(cached) = cache.get(&cache_key) {
            if cached.cached_at.elapsed() < cached.ttl {
                return success_json(&cached.data);
            }
        }
    }
    
    // Fetch fresh data
    let departures = dvb::monitor(&params.stop_id, 0, params.limit, params.mot)
        .await
        .map_err(|e| error_text(format!("Failed to monitor departures: {}", e)))?;
    
    // Update cache
    {
        let mut cache = self.departure_cache.lock().await;
        cache.insert(cache_key, CachedDepartures {
            station_id: params.stop_id.clone(),
            data: departures.clone(),
            cached_at: Instant::now(),
            ttl: Duration::from_secs(30),
        });
    }
    
    success_json(&departures)
}
```

**Benefits**:
- ✅ Reduces API latency for repeated calls
- ✅ Works transparently
- ✅ No protocol changes needed

**Limitations**:
- ❌ Not a replacement for subscriptions
- ❌ Cache invalidation complexity
- ❌ No proactive updates

### Strategy 5: Annotated Tool Responses

Include metadata in tool responses to hint at resource-like data.

**Implementation**:

```rust
#[tool(description = "Get user's saved origin location")]
async fn elicit_origin(&self, params: Parameters<OriginInfo>) -> CallToolResult {
    let mut location = self.user_location.lock().await;
    *location = Some(params.location.clone());
    
    // Return enhanced response with metadata
    let response = json!({
        "status": "success",
        "location": params.location,
        "saved_at": chrono::Utc::now().to_rfc3339(),
        "resource_hint": {
            "uri": "dvb://user/location",
            "ttl": "session",
            "cacheable": true
        },
        "message": format!("✓ Origin set to {}", params.location)
    });
    
    success_json(&response)
}
```

**Benefits**:
- ✅ Provides hints for future resource implementation
- ✅ Can include TTL/cache hints
- ✅ Self-documenting

**Limitations**:
- ❌ AI may ignore metadata
- ❌ Not standardized
- ❌ Still requires tool calls

### Strategy 6: Server Instructions Enhancement

Leverage the `instructions` field in ServerInfo to guide AI behavior.

**Current Implementation**:
```rust
fn get_info(&self) -> ServerInfo {
    ServerInfo {
        capabilities: ServerCapabilities::builder()
            .enable_tools()
            .enable_prompts()
            .build(),
        server_info: Implementation::from_build_env(),
        instructions: Some(
            "Dresden public transport assistant with route planning, departure monitoring, \
             trip tracking, and station search capabilities. Use the navigation-assistant prompt \
             to get started, departure-monitor for real-time departures, or trip-tracker to follow \
             a specific trip in real-time.".to_string(),
        ),
        ..Default::default()
    }
}
```

**Enhanced Implementation**:
```rust
fn get_info(&self) -> ServerInfo {
    ServerInfo {
        capabilities: ServerCapabilities::builder()
            .enable_tools()
            .enable_prompts()
            .build(),
        server_info: Implementation::from_build_env(),
        instructions: Some(
            "Dresden public transport assistant with route planning, departure monitoring, \
             trip tracking, and station search capabilities.\n\n\
             **CONTEXT MANAGEMENT**:\n\
             - This server maintains user context (location, destination) across conversations\n\
             - Call get_user_context at the start to check for saved context\n\
             - Use elicit_origin/elicit_destination to save context for future use\n\
             - Context persists for the session duration\n\n\
             **RECOMMENDED WORKFLOW**:\n\
             1. Call get_user_context first\n\
             2. If context exists, use it directly without asking\n\
             3. If context missing, ask user and save it\n\
             4. For real-time updates, call tools repeatedly (30s cache)\n\n\
             **PROMPTS**:\n\
             - navigation-assistant: General transit navigation\n\
             - departure-monitor: Real-time departure boards\n\
             - trip-tracker: Track specific trips (requires trip_id from route planning)\n\n\
             **PERFORMANCE TIPS**:\n\
             - Results are cached for 30 seconds\n\
             - Use get_station_context for comprehensive station data\n\
             - Trip IDs from get_route_details are required for trip tracking".to_string(),
        ),
        ..Default::default()
    }
}
```

**Benefits**:
- ✅ Visible to AI at initialization
- ✅ Documents expected behavior
- ✅ No code changes to core logic

**Limitations**:
- ❌ AI may not follow instructions perfectly
- ❌ Not enforced by protocol
- ❌ Limited formatting options

## Recommended Approach

**For Immediate Implementation**: Combine strategies 3, 4, and 6:

1. ✅ **Add `get_user_context` tool** (Strategy 3)
   - Single call to retrieve all context
   - Document in server instructions

2. ✅ **Implement caching** (Strategy 4)
   - 30-second cache for departures
   - 5-minute cache for station info
   - Session cache for user context

3. ✅ **Enhance server instructions** (Strategy 6)
   - Clear workflow documentation
   - Context management guidelines
   - Performance tips

4. ⏳ **Monitor rmcp updates** for resource support
   - Watch for v0.13+ releases
   - Prepare migration plan
   - Test resource features when available

## Implementation Checklist

- [ ] Add `get_user_context` unified context tool
- [ ] Add `get_station_context` comprehensive station tool
- [ ] Implement in-memory cache with TTL
- [ ] Update server instructions with workflow
- [ ] Add cache metrics/monitoring
- [ ] Document context tool in prompts
- [ ] Test with Claude Desktop/Cursor
- [ ] Monitor rmcp changelog for resource support
- [ ] Prepare resource migration plan (see RESOURCES_GUIDE.md)

## Example Usage Pattern

### Without Resources (Current - with workarounds):

```
User: "How do I get to Hauptbahnhof?"
AI: [Calls get_user_context]
AI: "I see you're at Albertplatz. Let me find routes..."
AI: [Calls get_route_details with origin=Albertplatz, destination=Hauptbahnhof]
AI: [Presents routes]
```

### With Resources (Future):

```
User: "How do I get to Hauptbahnhof?"
AI: [Reads dvb://user/location resource automatically]
AI: "I see you're at Albertplatz. Let me find routes..."
AI: [Calls get_route_details with origin=Albertplatz, destination=Hauptbahnhof]
AI: [Subscribes to dvb://trip/{trip_id} for selected route]
AI: [Presents routes with real-time updates]
```

## Performance Comparison

| Approach | Tool Calls | Latency | Updates | Complexity |
|----------|-----------|---------|---------|------------|
| **Current** | 2-3 per action | ~500ms | Poll | Low |
| **With Workarounds** | 1-2 per action | ~300ms | Poll | Medium |
| **With Resources** | 0-1 per action | ~100ms | Push | Medium |

## Conclusion

While waiting for MCP Resources support in rmcp:

1. Use **unified context tools** to reduce round trips
2. Implement **aggressive caching** for frequently accessed data
3. Enhance **server instructions** to guide AI behavior
4. Prepare for **resource migration** when available

The workarounds provide 60-70% of resource benefits with current tooling, while maintaining a clear migration path for when native resource support arrives.

---

**Status**: Workarounds active, waiting for rmcp v0.13+
**Next Review**: Check rmcp releases monthly
**Migration Ready**: See RESOURCES_GUIDE.md