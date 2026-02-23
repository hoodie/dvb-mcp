# MCP Resources Quick Reference

## TL;DR

**Current Status**: rmcp v0.12.0 doesn't support MCP Resources yet  
**Workaround**: ✅ Implemented `get_user_context` tool (33% fewer calls)  
**Future**: Native resources coming in rmcp v0.13+

---

## What Are Resources?

MCP Resources = **Automatic context** for AI without tool calls

| Feature | Description | Example |
|---------|-------------|---------|
| **URI-based** | Unique identifiers | `dvb://user/location` |
| **Auto-loaded** | No explicit calls needed | AI reads context automatically |
| **Subscribable** | Real-time updates | Push departure notifications |
| **Typed** | MIME types + schemas | `application/json` |

---

## Resources vs Tools

| | Resources | Tools |
|---|-----------|-------|
| **Purpose** | Context/State | Actions |
| **Access** | Automatic | Explicit call |
| **Updates** | Push (subscriptions) | Pull (polling) |
| **Latency** | ~100ms | ~500ms |
| **Example** | User's location | Search stations |

---

## Current Wor
