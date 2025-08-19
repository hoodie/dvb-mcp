use std::{future::Future, sync::Arc};

use anyhow::{Result, anyhow};
use dvb::{find_stops, point::Point};
use rmcp::{
    ErrorData as McpError, ServerHandler, elicit_safe,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::*,
    schemars::JsonSchema,
    service::{RequestContext, RoleServer},
    tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "User information")]
pub struct OriginInfo {
    #[schemars(description = "User's origin")]
    pub location: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "User destination information")]
pub struct DestinationInfo {
    #[schemars(description = "User's destination")]
    pub destination: String,
}

// Mark as safe for elicitation
elicit_safe!(OriginInfo);
elicit_safe!(DestinationInfo);

/// Simple tool request
#[derive(Debug, Deserialize, JsonSchema)]
pub struct LocateUser {
    pub location: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LinesRequest {
    pub start_query: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FindStationRequest {
    pub rough_stop_name: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FindNearbyStationRequest {
    pub rough_stop_name: String,
}

use chrono::DateTime;
use chrono::FixedOffset;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TripDetailsRequest {
    pub tripid: String,
    pub time: DateTime<FixedOffset>,
    pub stopid: String,
    pub mapdata: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RouteRequest {
    pub origin: String,
    pub destination: String,
    pub time: DateTime<chrono::Local>,
    pub isarrivaltime: Option<bool>,
    pub shorttermchanges: Option<bool>,
    pub format: Option<String>,
    pub via: Option<String>,
}

/// Simple server with elicitation
#[derive(Clone)]
pub struct DVBServer {
    user_location: Arc<Mutex<Option<String>>>,
    user_destination: Arc<Mutex<Option<String>>>,

    tool_router: ToolRouter<DVBServer>,
    prompts: Vec<Prompt>,
}

impl Default for DVBServer {
    fn default() -> Self {
        let prompts = vec![
            Prompt::new(
                "greeting",
                Some("A simple greeting prompt for Dresden transit assistant"),
                None,
            ),
            Prompt::new(
                "transit_workflow",
                Some("Guide the agent through the steps for planning a trip"),
                None,
            ),
        ];
        Self {
            user_location: Arc::new(Mutex::new(None)),
            user_destination: Arc::new(Mutex::new(None)),
            tool_router: Self::tool_router(),
            prompts,
        }
    }
}

fn success_text<S: Into<String>>(text: S) -> CallToolResult {
    CallToolResult::success(vec![Content::text(text.into())])
}

fn success_json<T: serde::Serialize>(data: &T) -> CallToolResult {
    CallToolResult::success(vec![Content::json(data).unwrap()])
}

fn error_text<S: Into<String>>(text: S) -> CallToolResult {
    CallToolResult::error(vec![Content::text(text.into())])
}

#[tool_router]
impl DVBServer {
    #[tool(
        description = "Ask the user's current location. This is to be used as the start point for trips. It is likely nearby a station."
    )]
    async fn elicit_origin(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let current_location = if let Some(location) = self.user_location.lock().await.clone() {
            location
        } else {
            match context
                .peer
                .elicit::<OriginInfo>("Please provide you are starting from".to_string())
                .await
            {
                Ok(Some(user_info)) => {
                    let location = user_info.location.clone();
                    *self.user_location.lock().await = Some(location.clone());
                    location
                }
                Ok(None) => "Hauptbahnhof Dresden".to_string(), // Never happen if client checks schema
                Err(_) => return Ok(error_text("unable to determine origin")),
            }
        };

        Ok(success_text(format!("Starting from {}!", current_location)))
    }

    #[tool(
        description = "Ask the user's destination. This is to be used as the end point for trips."
    )]
    async fn elicit_destination(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let current_destination =
            if let Some(destination) = self.user_destination.lock().await.clone() {
                destination
            } else {
                match context
                    .peer
                    .elicit::<DestinationInfo>("Please provide where you want to go.".to_string())
                    .await
                {
                    Ok(Some(dest_info)) => {
                        let destination = dest_info.destination.clone();
                        *self.user_destination.lock().await = Some(destination.clone());
                        destination
                    }
                    Ok(None) => "Hauptbahnhof Dresden".to_string(),
                    Err(_) => return Ok(error_text("unable to determine destination")),
                }
            };

        Ok(success_text(format!("Going to {}!", current_destination)))
    }

    #[tool(description = "Returns the current local time in ISO8601 (RFC3339) format.")]
    async fn now(&self) -> Result<CallToolResult, McpError> {
        let now = chrono::Local::now().to_rfc3339();
        Ok(success_text(now))
    }

    #[tool(
        description = "Clear the stored location for journey planning; will be requested again on next search."
    )]
    async fn reset_location(&self) -> Result<CallToolResult, McpError> {
        *self.user_location.lock().await = None;
        Ok(success_text(
            "User location reset. Next greeting will ask for name again.",
        ))
    }

    #[tool(
        description = "Search for tram stops, bus stops, or train stations in Dresden using a partial or approximate name."
    )]
    async fn find_stations(
        &self,
        Parameters(FindStationRequest { rough_stop_name }): Parameters<FindStationRequest>,
    ) -> Result<CallToolResult, McpError> {
        let found = match dvb::find_stops(&rough_stop_name).await {
            Ok(found) => found,
            Err(error) => {
                return Ok(error_text(format!(
                    "failed to find station {rough_stop_name:?} {error}"
                )));
            }
        };

        Ok(success_json(&*found))
    }

    #[tool(
        description = "Find tram stops, bus stops, or train stations near a specified location or landmark in Dresden."
    )]
    async fn find_nearby_stations(
        &self,
        Parameters(FindNearbyStationRequest { rough_stop_name }): Parameters<
            FindNearbyStationRequest,
        >,
    ) -> Result<CallToolResult, McpError> {
        let found = match dvb::find_nearby_stops(&rough_stop_name).await {
            Ok(found) => found,
            Err(error) => {
                return Ok(error_text(format!(
                    "failed to find nearby station {rough_stop_name:?} {error}"
                )));
            }
        };

        Ok(success_json(&*found))
    }

    #[tool(
        description = "List all tram, bus, or train lines departing from a specified stop or station in Dresden."
    )]
    async fn list_lines(
        &self,
        Parameters(LinesRequest { start_query }): Parameters<LinesRequest>,
    ) -> Result<CallToolResult, McpError> {
        let found = match dvb::find_stops(&start_query).await {
            Ok(found) => found,
            Err(error) => {
                return Ok(error_text(format!(
                    "failed to find stop {start_query:?} {error}"
                )));
            }
        };
        let start_point = match found.points.first() {
            Some(start_point) => start_point,

            None => {
                return Ok(error_text(format!("no search results for {start_query:?}")));
            }
        };

        let lines = match dvb::lines::lines(&start_point.id, None).await {
            Ok(resp) => resp.into_inner(),
            Err(error) => {
                return Ok(error_text(format!("failed to resolve lines {error}")));
            }
        };

        Ok(success_json(&lines))
    }

    #[tool(
        description = "Get detailed information for a specific trip, including all stops and times. Time must be an ISO8601 string."
    )]
    async fn get_trip_details(
        &self,
        Parameters(TripDetailsRequest {
            tripid,
            time,
            stopid,
            mapdata,
        }): Parameters<TripDetailsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let dvb_time = dvb::DvbTime::from(time);

        let params = dvb::trip::Params {
            tripid: &tripid,
            time: dvb_time,
            stopid: &stopid,
            mapdata,
        };

        let trip = match dvb::trip::trip_details(&params).await {
            Ok(resp) => resp,
            Err(e) => return Ok(error_text(format!("Failed to fetch trip details: {e}"))),
        };

        Ok(success_json(&*trip))
    }

    #[tool(
        description = "Query possible routes between two stops in Dresden. Returns possible trips, departure and arrival info, etc."
    )]
    async fn get_route_details(
        &self,
        Parameters(RouteRequest {
            origin,
            destination,
            time,
            isarrivaltime,
            shorttermchanges,
            format,
            via,
        }): Parameters<RouteRequest>,
    ) -> Result<CallToolResult, McpError> {
        let dvb_time = dvb::DvbTime::from(time);

        let origin_id = match lookup_stop_id(&origin).await {
            Ok(resp) => resp,
            Err(e) => {
                return Ok(error_text(format!("Failed to fetch origin id : {e}")));
            }
        };

        let destination_id = match lookup_stop_id(&destination).await {
            Ok(resp) => resp,
            Err(e) => {
                return Ok(error_text(format!("Failed to fetch destination id : {e}")));
            }
        };

        let params = dvb::route::Params {
            origin: &origin_id,
            destination: &destination_id,
            time: dvb_time,
            isarrivaltime: isarrivaltime.unwrap_or(false),
            shorttermchanges: shorttermchanges.unwrap_or(true),
            format: format.as_deref().unwrap_or("json"),
            via: via.as_deref(),
        };

        let route = match dvb::route::route_details(&params).await {
            Ok(resp) => resp,
            Err(e) => return Ok(error_text(format!("Failed to fetch route details: {e}"))),
        };

        Ok(success_json(&route))
    }
}

async fn lookup_stop_id(query: &str) -> anyhow::Result<String> {
    let found_origin = find_stops(&query).await?;
    let Point { id, .. } = found_origin
        .points
        .first()
        .ok_or_else(|| anyhow!("empty response"))?;
    Ok(id.to_owned())
}

#[tool_handler]
impl ServerHandler for DVBServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().enable_prompts().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Simple server demonstrating elicitation for user name collection and agent prompt support for Dresden public transport.".to_string(),
            ),
            ..Default::default()
        }
    }

    fn get_prompt(
        &self,
        request: GetPromptRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<rmcp::model::GetPromptResult, ErrorData>> + Send {
        let name = request.name.clone();
        async move {
            match name.as_str() {
                "greeting" => Ok(GetPromptResult {
                    description: Some(
                        "A simple greeting prompt for Dresden transit assistant".to_string(),
                    ),
                    messages: vec![
                        PromptMessage::new_text(
                            PromptMessageRole::User,
                            "Hello! How can I help you with Dresden public transport today?",
                        ),
                        PromptMessage::new_text(
                            PromptMessageRole::Assistant,
                            "Hi! I can help you find routes, departure times, and station info in Dresden. Where would you like to go?",
                        ),
                    ],
                }),
                "transit_workflow" => Ok(GetPromptResult {
                    description: Some(
                        "Guide the agent through the steps for planning a trip".to_string(),
                    ),
                    messages: vec![PromptMessage::new_text(
                        PromptMessageRole::Assistant,
                        "To plan your trip, I will first find the stop IDs for your origin and destination, then query possible routes, and finally present the best options with times and changes.",
                    )],
                }),
                _ => Err(McpError::invalid_params(
                    format!("Prompt '{}' not found", name),
                    None,
                )),
            }
        }
    }

    fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListPromptsResult, McpError>> + Send {
        let prompts = self.prompts.clone();
        async {
            Ok(ListPromptsResult {
                next_cursor: None,
                prompts,
            })
        }
    }
}
