use anyhow::{Result, anyhow};
use chrono::{DateTime, FixedOffset};
use rmcp::{
    ErrorData as McpError, ServerHandler, elicit_safe,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars::JsonSchema,
    serde_json,
    service::{RequestContext, RoleServer},
    tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use dvb::{find_stops, point::Point};
use std::{future::Future, sync::Arc};

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

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LinesRequest {
    // pub start_query: Option<String>,
    /// The ID of a point. Can be found via `lookup_point` function.
    pub point_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FindStationRequest {
    pub rough_stop_name: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FindNearbyStationRequest {
    pub rough_stop_name: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FindPoiRequest {
    /// Partial or full name of the point of interest to search for
    pub rough_poi_name: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MonitorDeparturesRequest {
    /// Partial or full stop name to search for
    pub stop_name: String,
    /// The ID of a point. Can be found via `lookup_point` function.
    pub stop_id: String,
    /// Optional list of modes of transport (e.g., ["Tram", "Bus"])
    pub mot: Option<Vec<String>>,
    /// Optional limit for number of departures
    pub limit: Option<u32>,
}
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TripDetailsRequest {
    pub trip_id: String,
    pub time: DateTime<FixedOffset>,
    /// The ID of a point. Can be found via `lookup_point` function.
    pub stop_id: String,
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

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct OsmLinkRequest {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct OsmLinkResponse {
    pub link: String,
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

    #[tool(description = "Returns a link to OpenStreetMap for the given coordinates.")]
    fn osm_link(
        &self,
        Parameters(OsmLinkRequest {
            latitude,
            longitude,
        }): Parameters<OsmLinkRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Basic validation
        if !(latitude >= -90.0 && latitude <= 90.0 && longitude >= -180.0 && longitude <= 180.0) {
            return Ok(error_text("Invalid latitude or longitude"));
        }
        let link = format!(
            "https://www.openstreetmap.org/?mlat={}&mlon={}&zoom=17",
            latitude, longitude
        );
        Ok(success_json(&OsmLinkResponse { link }))
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

    #[tool(description = "
        Search for points of interest (POIs) in Dresden using a partial or approximate name.
        Use this if you only get a rough description of a location or of where the user is to determine their location.
        ")]
    async fn find_pois(
        &self,
        Parameters(FindPoiRequest { rough_poi_name }): Parameters<FindPoiRequest>,
    ) -> Result<CallToolResult, McpError> {
        let found = match dvb::find_pois(&rough_poi_name).await {
            Ok(found) => found,
            Err(error) => {
                return Ok(error_text(format!(
                    "failed to find POI {rough_poi_name:?} {error}"
                )));
            }
        };

        Ok(success_json(&*found))
    }
    #[tool(description = "
        Get upcoming departures from a specified stop or station in Dresden.
        Optionally filter by mode of transport and limit the number of results.
        ")]
    async fn monitor_departures(
        &self,
        Parameters(MonitorDeparturesRequest {
            stop_name,
            stop_id,
            mot,
            limit,
        }): Parameters<MonitorDeparturesRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Parse Mot if provided
        let mot_filter = mot.as_ref().map(|mot_list| {
            mot_list
                .iter()
                .filter_map(|m| match m.as_str() {
                    "Tram" => Some(dvb::Mot::Tram),
                    "Bus" => Some(dvb::Mot::Bus),
                    "Ferry" => Some(dvb::Mot::Ferry),
                    "Train" => Some(dvb::Mot::Train),
                    _ => None,
                })
                .collect::<Vec<_>>()
        });

        let monitor_params = dvb::monitor::Params {
            stopid: &stop_id,
            mot: mot_filter.as_ref().map(|v| v.as_slice()),
            limit,
            ..Default::default()
        };

        let departures = match dvb::monitor::departure_monitor(monitor_params).await {
            Ok(deps) => deps,
            Err(error) => {
                return Ok(error_text(format!(
                    "failed to fetch departures for {stop_id}({stop_name:?}) {error}"
                )));
            }
        };

        Ok(success_json(&departures))
    }
    #[tool(
        description = "List all tram, bus, or train lines departing from a specified stop or station in Dresden."
    )]
    async fn list_lines(
        &self,
        Parameters(LinesRequest { point_id }): Parameters<LinesRequest>,
    ) -> Result<CallToolResult, McpError> {
        let start_point_id = if let Some(point_id) = point_id {
            point_id
        } else {
            return Ok(error_text("missing start point"));
        };
        let lines = match dvb::lines::lines(&start_point_id, None).await {
            Ok(resp) => resp.into_inner(),
            Err(error) => {
                return Ok(error_text(format!("failed to resolve lines {error}")));
            }
        };

        Ok(success_json(&lines))
    }

    #[tool(description = "
        Get detailed information for a specific trip, including all stops and times.
        Time must be an ISO8601 string.")]
    async fn get_trip_details(
        &self,
        Parameters(TripDetailsRequest {
            trip_id,
            time,
            stop_id,
            mapdata,
        }): Parameters<TripDetailsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let dvb_time = dvb::DvbTime::from(time);

        let params = dvb::trip::Params {
            tripid: &trip_id,
            time: dvb_time,
            stopid: &stop_id,
            mapdata,
        };

        let trip = match dvb::trip::trip_details(&params).await {
            Ok(resp) => resp,
            Err(e) => return Ok(error_text(format!("Failed to fetch trip details: {e}"))),
        };

        Ok(success_json(&*trip))
    }

    #[tool(description = "
        Query possible routes between two stops in Dresden.
        Returns possible trips, departure and arrival info, etc.")]
    async fn get_route_details(
        &self,
        Parameters(RouteRequest {
            origin,      // TODO: replace with origin_id
            destination, // TODO: replace with destination_id
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

        // Strip out partial_routes from each Route before returning
        let mut routes = route.into_inner();
        for r in &mut routes.routes {
            r.partial_routes = None;
        }

        Ok(success_json(&routes))
    }

    #[tool(
        description = "Look up the stop ID for a given stop name or query string in Dresden. Returns the stop ID if found."
    )]
    async fn lookup_stop_id_tool(
        &self,
        Parameters(FindStationRequest { rough_stop_name }): Parameters<FindStationRequest>,
    ) -> Result<CallToolResult, McpError> {
        let found = match dvb::find_stops(&rough_stop_name).await {
            Ok(found) => found,
            Err(error) => {
                return Ok(error_text(format!(
                    "failed to find stop {rough_stop_name:?} {error}"
                )));
            }
        };
        let stop = match found.points.first() {
            Some(stop) => stop,
            None => {
                return Ok(error_text(format!(
                    "no search results for {rough_stop_name:?}"
                )));
            }
        };

        Ok(success_json(&serde_json::json!({ "stop_id": stop.id })))
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
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_prompts()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(String::from(
                "Simple server demonstrating elicitation for user name collection and agent prompt support for Dresden public transport.",
            )),
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
