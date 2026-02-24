use anyhow::Result;
use rmcp::{
    ErrorData as McpError, ServerHandler,
    model::*,
    prompt_handler, serde_json,
    service::{RequestContext, RoleServer},
    tool_handler,
};

use crate::server::{DVBServer, usercontext::UserContext};

#[tool_handler]
#[prompt_handler]
impl ServerHandler for DVBServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
        capabilities: ServerCapabilities::builder()
            .enable_tools()
            .enable_prompts()
            .enable_resources()
            .build(),
        server_info: Implementation::from_build_env(),
        instructions: Some(
            "Dresden public transport assistant with route planning, departure monitoring, \
                 trip tracking, and station search capabilities.\n\n\
                 **CONTEXT MANAGEMENT**:\n\
                 - This server provides RESOURCES for automatic context access\n\
                 - Available resources: dvb://user/context, dvb://user/location, dvb://user/destination\n\
                 - Resources are automatically available - no tool call needed!\n\
                 - For backward compatibility, get_user_context tool is also available\n\
                 - Use elicit_origin/elicit_destination to save context for future use\n\
                 - Context persists for the session duration\n\n\
                 **RECOMMENDED WORKFLOW**:\n\
                 1. Read dvb://user/context resource to check existing context (automatic!)\n\
                 2. If context exists, use it directly without asking redundant questions\n\
                 3. If context missing, ask user and save via elicit_origin/elicit_destination\n\
                 4. For real-time updates, call tools as needed\n\n\
                 **RESOURCES**:\n\
                 - dvb://user/context: Complete user context (location + destination + status)\n\
                 - dvb://user/location: Current user location (when set)\n\
                 - dvb://user/destination: Current user destination (when set)\n\n\
                 **PROMPTS**:\n\
                 - navigation-assistant: General transit navigation and route planning\n\
                 - departure-monitor: Real-time departure boards for stations\n\
                 - trip-tracker: Track specific trips in real-time (requires trip_id from route planning)\n\n\
                 **TRIP TRACKING NOTE**:\n\
                 Trip tracking requires a trip_id obtained from get_route_details. Store trip IDs \
                 in conversation context to provide updates when user asks about their journey.".to_string(),
        ),
        ..Default::default()
    }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        let origin = self.user_origin.lock().await.clone();
        let location = self.user_location.lock().await.clone();
        let destination = self.user_destination.lock().await.clone();

        let mut resources = vec![
            RawResource::new("dvb://user/context", "User Context".to_string()).no_annotation(),
        ];

        // Add origin resource if set
        if origin.is_some() {
            resources.push(
                RawResource::new("dvb://user/origin", "User Origin".to_string()).no_annotation(),
            );
        }

        // Add location resource if set
        if location.is_some() {
            resources.push(
                RawResource::new("dvb://user/location", "User Current Location".to_string())
                    .no_annotation(),
            );
        }

        // Add destination resource if set
        if destination.is_some() {
            resources.push(
                RawResource::new("dvb://user/destination", "User Destination".to_string())
                    .no_annotation(),
            );
        }

        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
            meta: None,
        })
    }

    async fn read_resource(
        &self,
        ReadResourceRequestParams { uri, .. }: ReadResourceRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        match uri.as_str() {
            "dvb://user/context" => {
                let origin = self.user_origin.lock().await.clone();
                let location = self.user_location.lock().await.clone();
                let destination = self.user_destination.lock().await.clone();

                let context = UserContext::new(origin, location, destination);

                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(
                        serde_json::to_string_pretty(&context).unwrap(),
                        uri,
                    )],
                })
            }
            "dvb://user/origin" => {
                let origin = self.user_origin.lock().await.clone();

                match origin {
                    Some(org) => {
                        let data = serde_json::json!({
                            "origin": org,
                            "last_updated": chrono::Local::now().to_rfc3339(),
                        });

                        Ok(ReadResourceResult {
                            contents: vec![ResourceContents::text(
                                serde_json::to_string_pretty(&data).unwrap(),
                                uri,
                            )],
                        })
                    }
                    None => Err(McpError::resource_not_found(
                        "Origin not set",
                        Some(serde_json::json!({ "uri": uri })),
                    )),
                }
            }
            "dvb://user/location" => {
                let location = self.user_location.lock().await.clone();

                match location {
                    Some(loc) => {
                        let data = serde_json::json!({
                            "location": loc,
                            "last_updated": chrono::Local::now().to_rfc3339(),
                        });

                        Ok(ReadResourceResult {
                            contents: vec![ResourceContents::text(
                                serde_json::to_string_pretty(&data).unwrap(),
                                uri,
                            )],
                        })
                    }
                    None => Err(McpError::resource_not_found(
                        "Current location not set",
                        Some(serde_json::json!({ "uri": uri })),
                    )),
                }
            }
            "dvb://user/destination" => {
                let destination = self.user_destination.lock().await.clone();

                match destination {
                    Some(dest) => {
                        let data = serde_json::json!({
                            "destination": dest,
                            "last_updated": chrono::Local::now().to_rfc3339(),
                        });

                        Ok(ReadResourceResult {
                            contents: vec![ResourceContents::text(
                                serde_json::to_string_pretty(&data).unwrap(),
                                uri,
                            )],
                        })
                    }
                    None => Err(McpError::resource_not_found(
                        "Destination not set",
                        Some(serde_json::json!({ "uri": uri })),
                    )),
                }
            }
            _ => {
                // Check if it's a departures resource with pattern dvb://departures/{stop_id}
                if uri.starts_with("dvb://departures/") {
                    let stop_id = uri.strip_prefix("dvb://departures/").unwrap();

                    // Fetch departures using dvb crate
                    let monitor_params = dvb::monitor::Params {
                        stopid: stop_id,
                        mot: None,
                        limit: Some(10),
                        ..Default::default()
                    };

                    match dvb::monitor::departure_monitor(monitor_params).await {
                        Ok(departures) => {
                            let data = serde_json::json!({
                                "stop_id": stop_id,
                                "departures": departures,
                                "last_updated": chrono::Local::now().to_rfc3339(),
                            });

                            Ok(ReadResourceResult {
                                contents: vec![ResourceContents::text(
                                    serde_json::to_string_pretty(&data).unwrap(),
                                    uri,
                                )],
                            })
                        }
                        Err(error) => Err(McpError::resource_not_found(
                            format!(
                                "Failed to fetch departures for stop_id {}: {}",
                                stop_id, error
                            ),
                            Some(serde_json::json!({ "uri": uri, "stop_id": stop_id })),
                        )),
                    }
                } else {
                    Err(McpError::resource_not_found(
                        "Resource not found",
                        Some(serde_json::json!({ "uri": uri })),
                    ))
                }
            }
        }
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        let templates = vec![
        RawResourceTemplate {
            uri_template: "dvb://departures/{stop_id}".to_string(),
            name: "Station Departures".to_string(),
            title: Some("Real-time Departures".to_string()),
            description: Some("Real-time departure information for a specific stop. Use the stop_id from find_stations or lookup_stop_id.".to_string()),
            mime_type: Some("application/json".to_string()),
            icons: None,
        }.no_annotation(),
    ];

        Ok(ListResourceTemplatesResult {
            next_cursor: None,
            resource_templates: templates,
            meta: None,
        })
    }
}
