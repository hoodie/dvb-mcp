# Future Prompts for DVB MCP Server

This document outlines potential prompts that could be added to enhance the DVB MCP server's functionality. Each prompt targets specific use cases and user scenarios.

## Current Implementation

### ✅ navigation-assistant
**Status:** Implemented

**Description:** General-purpose interactive assistant for navigating Dresden's public transportation system.

**Use Cases:**
- First-time users needing comprehensive journey planning
- Multi-step trip planning with origin and destination
- General public transport assistance

**Key Features:**
- Elicits user origin and destination
- Provides route planning with connections
- Offers comprehensive transit information

---

### ✅ departure-monitor
**Status:** Implemented

**Description:** Real-time departure board for checking when the next vehicles are leaving from a specific station.

**Use Cases:**
- "When's the next tram from Hauptbahnhof?"
- Commuters at their regular stop checking next departure
- Quick check without full journey planning
- Monitoring specific lines or directions

**Key Features:**
- Quick real-time departure information
- Shows line number, destination, and departure time
- Highlights delays and disruptions
- Includes platform/bay information when available

---

## Proposed Future Prompts

### 1. trip-tracker

**Status:** ✅ Implemented

**Priority:** Low → High (Promoted)

**Description:** Get detailed real-time information about a specific trip in progress.

**Use Cases:**
- Following a trip with multiple connections
- Checking if connection is still valid
- Real-time delay information
- Stop-by-stop progress

**Tools Utilized:**
- `get_trip_details` - Primary tool for trip tracking
- `monitor_departures` - Check connection validity
- `now` - Current time for progress tracking

**Sample Conversation Flow:**
```
User: "Track my trip on tram 3 departing at 14:12"
Assistant: "Tracking tram 3 trip:
  
  Current status: On time
  
  Upcoming stops:
  ✓ Albertplatz (14:12) - Departed
  → Carolaplatz (14:15) - Next stop
  - Pirnaischer Platz (14:17)
  - Hauptbahnhof (14:20)
  
  Your connection to line 7 at Hauptbahnhof is secure."
```

**Implementation Notes:**
- LLM keeps trip ID in conversation context
- Visual progress indication with checkmarks and arrows
- Alerts on delays affecting connections
- Platform/track information when available
- No backend state storage needed - context-based tracking

---

### 2. arrival-planner

**Priority:** High

**Description:** Real-time departure board for checking when the next vehicles are leaving from a specific station.

**Use Cases:**
- "When's the next tram from Hauptbahnhof?"
- Commuters at their regular stop checking next departure
- Quick check without full journey planning
- Monitoring specific lines or directions

**Tools Utilized:**
- `monitor_departures` - Primary tool for real-time departures
- `find_stations` - Resolve station names to IDs
- `lookup_stop_id` - Quick station lookup
- `now` - Current time reference

**Sample Conversation Flow:**
```
User: "Show me the next departures from Postplatz"
Assistant: "I'll check the real-time departures at Postplatz..."
[Uses find_stations → lookup_stop_id → monitor_departures]
Assistant: "Here are the next departures:
  - Line 1 to Prohlis: 2 minutes
  - Line 2 to Gorbitz: 5 minutes
  - Line 4 to Weinböhla: 7 minutes"
```

**Implementation Notes:**
- Focus on speed and clarity
- Default to showing all modes of transport unless filtered
- Include platform/bay information if available
- Highlight delays or disruptions

---

**Description:** Journey planning when you need to arrive by a specific time, working backwards from a deadline.

**Use Cases:**
- "I need to be at the airport by 3 PM"
- Meeting deadlines with arrival time constraints
- Calculating latest departure time
- Planning with time buffers

**Tools Utilized:**
- `get_route_details` - With `isarrivaltime: true`
- `elicit_origin` - Starting location
- `elicit_destination` - Target location
- `now` - Check if deadline is feasible

**Sample Conversation Flow:**
```
User: "I need to arrive at Dresden Airport by 14:30"
Assistant: "Where are you starting from?"
User: "Neustadt Bahnhof"
Assistant: "To arrive by 14:30, you should take:
  - Depart at 13:45 - S2 from Neustadt
  - Arrive at 14:22 (8 minutes buffer)
  
  Alternative with more buffer:
  - Depart at 13:25 - S2 from Neustadt
  - Arrive at 14:02 (28 minutes buffer)"
```

**Implementation Notes:**
- Emphasize arrival time guarantees
- Suggest routes with time buffers
- Warn if deadline seems tight
- Consider showing multiple options with different buffer times

---

### 2. poi-explorer

**Priority:** Medium

**Description:** Discover points of interest in Dresden and plan how to reach them using public transport.

**Use Cases:**
- Tourist exploration ("What's interesting near here?")
- Finding specific POI types (museums, restaurants, parks)
- Combined discovery and navigation
- Event venue location and access

**Tools Utilized:**
- `find_pois` - Search for points of interest
- `osm_link` - Provide map links for discovered POIs
- `get_route_details` - Plan journey to selected POI
- `elicit_origin` - User's starting point
- `find_nearby_stations` - Stations near POIs

**Sample Conversation Flow:**
```
User: "What museums can I visit in Dresden?"
Assistant: "I found several museums:
  - Zwinger Palace - [map link]
  - Green Vault - [map link]
  - German Hygiene Museum - [map link]
  
  Which one interests you?"
User: "The Zwinger Palace"
Assistant: "Great choice! Where are you starting from?"
[Plans route to nearest stop]
```

**Implementation Notes:**
- Integrate POI search with navigation
- Provide context about POIs (brief descriptions if available)
- Offer multiple options when relevant
- Include walking distance from nearest stop

---

### 3. line-explorer

**Priority:** Medium

**Description:** Explore specific tram, bus, or train lines to understand their routes and schedules.

**Use Cases:**
- "Tell me about tram line 3"
- Understanding line coverage
- Finding stops along a specific line
- Planning trips along a single line

**Tools Utilized:**
- `list_lines` - Get lines at various stops
- `get_trip_details` - Detailed trip information for a line
- `monitor_departures` - Frequency and timing
- `find_stations` - Locate stops along the line

**Sample Conversation Flow:**
```
User: "Tell me about tram line 11"
Assistant: "Tram line 11 runs between Zschertnitz and Bühlau.
  
  Key stops include:
  - Hauptbahnhof (main station)
  - Postplatz (city center)
  - Albertplatz
  
  Next departures from Hauptbahnhof:
  - 14:15, 14:25, 14:35 (every 10 minutes)
  
  Would you like to plan a trip on this line?"
```

**Implementation Notes:**
- Provide route overview
- Show frequency and patterns
- Highlight major stops/connections
- Option to drill into specific stops

---

### 4. quick-lookup

**Priority:** Low

**Description:** Fast information lookup about stations, stops, or locations without full journey planning.

**Use Cases:**
- "Where is Postplatz?"
- "What lines stop at Albertplatz?"
- Quick fact finding
- Location verification

**Tools Utilized:**
- `find_stations` - Locate stations
- `find_pois` - Find landmarks
- `list_lines` - Lines at a stop
- `osm_link` - Map location
- `find_nearby_stations` - Nearby stops

**Sample Conversation Flow:**
```
User: "What's at Albertplatz?"
Assistant: "Albertplatz is a major tram junction in Neustadt.
  
  Lines serving this stop:
  - Trams: 3, 6, 7, 8, 11, 13
  
  Nearby landmarks:
  - Kunsthofpassage (art courtyard)
  - Neustadt district center
  
  [OpenStreetMap link]
  
  Would you like to plan a trip to or from here?"
```

**Implementation Notes:**
- Fast, factual responses
- No journey planning unless requested
- Rich context about the location
- Transition to navigation if needed

---

## Implementation Priority

1. **High Priority:** arrival-planner
   - Addresses time-critical journey planning
   - Complements existing navigation-assistant, departure-monitor, and trip-tracker

2. **Medium Priority:** poi-explorer, line-explorer
   - Add value for tourists and explorers
   - Leverage underutilized tools (find_pois, list_lines)

3. **Low Priority:** quick-lookup
   - Nice-to-have feature
   - Can be handled by existing prompts with guidance

---

## Technical Considerations

### Prompt Design Pattern

Each prompt should follow this structure:

```rust
#[prompt(
    name = "prompt-name",
    description = "Clear description of prompt purpose and use case"
)]
async fn prompt_name(&self) -> Vec<PromptMessage> {
    vec![
        PromptMessage::new_text(
            PromptMessageRole::Assistant,
            "System instruction: Define the assistant's role and constraints",
        ),
        PromptMessage::new_text(
            PromptMessageRole::User,
            "Example user request",
        ),
        PromptMessage::new_text(
            PromptMessageRole::Assistant,
            "Example assistant response showing desired behavior",
        ),
    ]
}
```

### Multi-Turn Conversations

Prompts should guide multi-turn interactions:
- Ask clarifying questions when needed
- Remember context from earlier in the conversation
- Offer relevant follow-up actions

### Error Handling

Each prompt should handle:
- Station/stop not found
- No routes available
- Time constraints impossible to meet
- Service disruptions

### User Experience

- Keep responses concise but informative
- Use structured formatting for readability
- Provide actionable next steps
- Link to maps when relevant

---

## Testing Scenarios

For each implemented prompt, test:

1. **Happy path:** Typical use case with valid inputs
2. **Disambiguation:** Multiple matches requiring user choice
3. **Not found:** Invalid station/POI names
4. **Edge cases:** Very early/late times, same origin/destination
5. **Complex scenarios:** Multiple connections, delays

---

## Future Enhancements

### Potential Advanced Features

- **commute-optimizer:** Save and optimize regular routes
- **accessibility-helper:** Focus on step-free access, elevators
- **disruption-alert:** Subscribe to line/route updates
- **multi-stop-planner:** Plan trips with multiple waypoints
- **price-calculator:** Estimate ticket costs for journeys
- **bike-transit-combo:** Combine cycling with public transport

### Integration Opportunities

- Weather-aware suggestions
- Real-time crowding information
- Historical delay patterns
- Event-based routing (concerts, football matches)

---

## Contributing

When adding new prompts:

1. Document the use case clearly
2. List all tools utilized
3. Provide sample conversation flows
4. Consider edge cases
5. Update this document
6. Add tests for the prompt

---

**Last Updated:** 2024-12-19  
**Version:** 1.2