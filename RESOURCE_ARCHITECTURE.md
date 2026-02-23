# DVB MCP Server - Resource Architecture

## Overview

This document provides a visual overview of how resources are implemented in the DVB MCP server.

## Resource Types

```
┌─────────────────────────────────────────────────────────────────┐
│                    DVB MCP Resources                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Static Resources (Always Available)                            │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ dvb://user/context                                        │  │
│  │ - Returns: origin, location, destination, status         │  │
│  │ - Always present (can be empty)                          │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
│  Conditional Resources (When Data Set)                          │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ dvb://user/location                                       │  │
│  │ - Returns: current location                              │  │
│  │ - Only appears when location is set                      │  │
│  └──────────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ dvb://user/destination                                    │  │
│  │ - Returns: destination point                             │  │
│  │ - Only appears when destination is set                   │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
│  Resource Templates (Parameterized)                             │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ dvb://departures/{stop_id}                                │  │
│  │ - Parameter: stop_id (e.g., "33000001")                  │  │
│  │ - Returns: real-time departures for the stop             │  │
│  │ - Example: dvb://departures/33000001                     │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Request Flow

### Listing Resources

```
┌──────────┐                                    ┌──────────────┐
│          │ resources/list                     │              │
│  Client  │ ──────────────────────────────────>│  DVB Server  │
│          │                                    │              │
└──────────┘                                    └──────────────┘
                                                       │
                                                       │ Check State
                                                       ▼
                                                ┌─────────────┐
                                                │ user_origin │
                                                │ user_location│
                                                │ user_dest   │
                                                └─────────────┘
                                                       │
                                                       │ Build List
                                                       ▼
┌──────────┐                                    ┌──────────────┐
│          │ [dvb://user/context,               │              │
│  Client  │ <────────────────── dvb://user/location] ────────│  DVB Server  │
│          │                                    │              │
└──────────┘                                    └──────────────┘
```

### Reading a Static Resource

```
┌──────────┐                                    ┌──────────────┐
│          │ resources/read                     │              │
│  Client  │ { "uri": "dvb://user/context" }    │              │
│          │ ──────────────────────────────────>│  DVB Server  │
│          │                                    │              │
└──────────┘                                    └──────────────┘
                                                       │
                                                       │ Read State
                                                       ▼
                                                ┌─────────────┐
                                                │ Mutex locks │
                                                │ user_origin │
                                                │ user_dest   │
                                                └─────────────┘
                                                       │
                                                       │ Build JSON
                                                       ▼
┌──────────┐                                    ┌──────────────┐
│          │ { "location": "Albertplatz", ...}  │              │
│  Client  │ <───────────────────────────────────│  DVB Server  │
│          │                                    │              │
└──────────┘                                    └──────────────┘
```

### Reading a Template Resource

```
┌──────────┐                                    ┌──────────────┐
│          │ resources/read                     │              │
│  Client  │ {"uri": "dvb://departures/33000001"}│              │
│          │ ──────────────────────────────────>│  DVB Server  │
│          │                                    │              │
└──────────┘                                    └──────────────┘
                                                       │
                                                       │ Parse URI
                                                       │ Extract: stop_id="33000001"
                                                       ▼
                                                ┌──────────────┐
                                                │ dvb::monitor │
                                                │ ::departure_ │
                                                │ monitor()    │
                                                └──────────────┘
                                                       │
                                                       │ API Call
                                                       ▼
                                                ┌──────────────┐
                                                │ DVB API      │
                                                │ (Real-time)  │
                                                └──────────────┘
                                                       │
                                                       │ Departures Data
                                                       ▼
┌──────────┐                                    ┌──────────────┐
│          │ { "stop_id": "33000001",           │              │
│  Client  │   "departures": [...], ... }       │              │
│          │ <───────────────────────────────────│  DVB Server  │
│          │                                    │              │
└──────────┘                                    └──────────────┘
```

## Implementation Architecture

```
┌────────────────────────────────────────────────────────────────┐
│                         DVBServer                               │
├────────────────────────────────────────────────────────────────┤
│                                                                 │
│  State Management                                              │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │ Arc<Mutex<Option<String>>> user_origin                   │ │
│  │ Arc<Mutex<Option<String>>> user_location                 │ │
│  │ Arc<Mutex<Option<String>>> user_destination              │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                 │
│  Resource Handlers                                             │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │ list_resources()                                          │ │
│  │ ├─ Check state locks                                     │ │
│  │ ├─ Build dynamic resource list                           │ │
│  │ └─ Return RawResource vector                             │ │
│  │                                                           │ │
│  │ read_resource(uri)                                        │ │
│  │ ├─ Match on URI pattern                                  │ │
│  │ │  ├─ "dvb://user/context" → UserContext                 │ │
│  │ │  ├─ "dvb://user/location" → location state             │ │
│  │ │  ├─ "dvb://user/destination" → destination state       │ │
│  │ │  └─ "dvb://departures/*" → fetch departures            │ │
│  │ └─ Return ResourceContents                               │ │
│  │                                                           │ │
│  │ list_resource_templates()                                 │ │
│  │ └─ Return RawResourceTemplate vector                     │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                 │
│  External APIs                                                 │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │ dvb::monitor::departure_monitor()                         │ │
│  │ - Called for dvb://departures/{stop_id}                  │ │
│  │ - Returns real-time departure data                       │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                 │
└────────────────────────────────────────────────────────────────┘
```

## Data Flow

### User Context Update

```
Tool Call                Resource Access
    │                         │
    ▼                         ▼
set_location()         dvb://user/location
    │                         │
    ▼                         │
Lock & Update ─────────────>  │
user_location                 │
    │                         │
    │                    Lock & Read
    │                    user_location
    │                         │
    ▼                         ▼
Return success         Return JSON response
```

### Departure Query

```
Resource Request: dvb://departures/33000001
           │
           ▼
    Parse URI pattern
    Extract stop_id = "33000001"
           │
           ▼
    dvb::monitor::Params {
        stopid: "33000001",
        mot: None,
        limit: Some(10),
    }
           │
           ▼
    dvb::monitor::departure_monitor()
           │
           ▼
    DVB API (HTTP)
           │
           ▼
    Parse Departures
           │
           ▼
    Build JSON:
    {
      "stop_id": "33000001",
      "departures": [...],
      "last_updated": "..."
    }
           │
           ▼
    ResourceContents::text()
           │
           ▼
    Return to Client
```

## Error Handling

```
Resource Request
      │
      ▼
   URI Valid?
      │
      ├─ No ──> McpError::resource_not_found
      │
      ▼ Yes
   Check Resource Type
      │
      ├─ Static Resource
      │     │
      │     ▼
      │  State Exists?
      │     │
      │     ├─ Yes ──> Return State
      │     └─ No ──> McpError::resource_not_found
      │
      └─ Template Resource
            │
            ▼
         API Call Success?
            │
            ├─ Yes ──> Return Data
            └─ No ──> McpError::resource_not_found
                         with error details
```

## Performance Characteristics

| Resource Type          | Latency      | Dependencies       |
|------------------------|--------------|-------------------|
| dvb://user/context     | ~1ms         | Memory (Mutex)    |
| dvb://user/location    | ~1ms         | Memory (Mutex)    |
| dvb://user/destination | ~1ms         | Memory (Mutex)    |
| dvb://departures/*     | ~200-500ms   | DVB API (HTTP)    |

## Future Architecture

```
Current:
  Client → read_resource() → DVB API → Response

Future (with subscriptions):
  Client → subscribe(uri) → DVB Server
                              │
                              ├─ Polling Loop (30s)
                              │    │
                              │    ▼
                              │  DVB API
                              │    │
                              │    ▼
                              │  Detect Changes
                              │    │
                              ▼    ▼
                          Push Updates → Client
```

---

**Last Updated**: 2024-12-19
**Version**: 0.1.0
**Status**: Current Implementation
