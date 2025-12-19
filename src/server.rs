use anyhow::{Result, anyhow};
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{
        router::{prompt::PromptRouter, tool::ToolRouter},
        wrapper::Parameters,
    },
    model::*,
    prompt, prompt_handler, prompt_router, serde_json,
    service::{RequestContext, RoleServer},
    tool, tool_handler, tool_router,
};
use tokio::sync::Mutex;

use dvb::{find_stops, point::Point};
use std::sync::Arc;

mod args {
    //! Argument types for MCP tools

    use chrono::{DateTime, FixedOffset};
    use rmcp::{elicit_safe, schemars::JsonSchema};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    #[schemars(description = "User origin (journey starting point) information")]
    pub struct OriginInfo {
        #[schemars(description = "User's journey origin/starting point")]
        pub origin: String,
    }
    // Mark as safe for elicitation
    elicit_safe!(OriginInfo);

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    #[schemars(description = "User current location information")]
    pub struct LocationInfo {
        #[schemars(description = "User's current location")]
        pub location: String,
    }
    // Mark as safe for elicitation
    elicit_safe!(LocationInfo);

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    #[schemars(description = "User destination information")]
    pub struct DestinationInfo {
        #[schemars(description = "User's destination")]
        pub destination: String,
    }
    // Mark as safe for elicitation
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
}

use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct OsmLinkResponse {
    pub link: String,
}

/// Status of user context completeness
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum UserContextStatus {
    /// All three fields (origin, location, destination) are set
    Complete,
    /// At least one field is set, but not all
    Partial,
    /// No fields are set
    Empty,
}

/// User context containing origin, current location, and destination
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct UserContext {
    /// Where the user's journey starts
    pub origin: Option<String>,
    /// Where the user currently is
    pub location: Option<String>,
    /// Where the user wants to go
    pub destination: Option<String>,
    /// Last time context was updated
    pub last_updated: String,
    /// Whether any context is available
    pub context_available: bool,
    /// Status of context completeness
    pub status: UserContextStatus,
    /// Human-readable description of the context
    pub message: String,
}

impl UserContext {
    /// Create a new UserContext from the given values
    pub fn new(
        origin: Option<String>,
        location: Option<String>,
        destination: Option<String>,
    ) -> Self {
        let any_set = origin.is_some() || location.is_some() || destination.is_some();
        let all_set = origin.is_some() && location.is_some() && destination.is_some();

        let status = if all_set {
            UserContextStatus::Complete
        } else if any_set {
            UserContextStatus::Partial
        } else {
            UserContextStatus::Empty
        };

        let message = match (&origin, &location, &destination) {
            (Some(org), Some(loc), Some(dest)) => {
                format!(
                    "User journey: Origin: {}, Current location: {}, Destination: {}",
                    org, loc, dest
                )
            }
            (Some(org), Some(loc), None) => {
                format!(
                    "User origin: {}, Current location: {} (destination not set)",
                    org, loc
                )
            }
            (Some(org), None, Some(dest)) => {
                format!(
                    "User origin: {}, Destination: {} (current location not set)",
                    org, dest
                )
            }
            (None, Some(loc), Some(dest)) => {
                format!(
                    "Current location: {}, Destination: {} (origin not set)",
                    loc, dest
                )
            }
            (Some(org), None, None) => {
                format!("User origin: {} (location and destination not set)", org)
            }
            (None, Some(loc), None) => {
                format!("Current location: {} (origin and destination not set)", loc)
            }
            (None, None, Some(dest)) => {
                format!("Destination: {} (origin and location not set)", dest)
            }
            (None, None, None) => "No user context saved yet".to_string(),
        };

        Self {
            origin,
            location,
            destination,
            last_updated: chrono::Local::now().to_rfc3339(),
            context_available: any_set,
            status,
            message,
        }
    }
}

/// Simple server with elicitation
#[derive(Clone)]
pub struct DVBServer {
    user_origin: Arc<Mutex<Option<String>>>,
    user_location: Arc<Mutex<Option<String>>>,
    user_destination: Arc<Mutex<Option<String>>>,

    tool_router: ToolRouter<DVBServer>,
    prompt_router: PromptRouter<DVBServer>,
}

impl Default for DVBServer {
    fn default() -> Self {
        Self {
            user_origin: Arc::new(Mutex::new(None)),
            user_location: Arc::new(Mutex::new(None)),
            user_destination: Arc::new(Mutex::new(None)),
            tool_router: Self::tool_router(),
            prompt_router: Self::prompt_router(),
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

#[prompt_router]
impl DVBServer {
    /// Dresden public transport navigation assistant
    #[prompt(
        name = "navigation-assistant",
        description = "Interactive assistant for navigating Dresden's public transportation system"
    )]
    async fn navigation_assistant(&self) -> Vec<PromptMessage> {
        vec![
            PromptMessage::new_text(
                PromptMessageRole::Assistant,
                "You are a travel assistant for Dresden's public transportation system (DVB). \
                 You have access to tools from the local public transportation provider. \
                 You can help users navigate the city by finding stations, checking departure times, \
                 and planning routes. Use the available tools to provide a pleasant experience. \
                 When asked for navigation assistance, first determine the user's origin and destination, \
                 then use the route planning tools to find the best connections.",
            ),
            PromptMessage::new_text(
                PromptMessageRole::User,
                "I need help getting around Dresden using public transport.",
            ),
            PromptMessage::new_text(
                PromptMessageRole::Assistant,
                "I'd be happy to help you navigate Dresden's public transportation! \
                 To get started, I'll need to know where you are and where you'd like to go. \
                 I can help you find nearby stations, check departure times, and plan your route. \
                 What's your journey today?",
            ),
        ]
    }

    /// Real-time departure monitor for Dresden public transport stops
    #[prompt(
        name = "departure-monitor",
        description = "Real-time departure board for checking when the next vehicles are leaving from a specific station"
    )]
    async fn departure_monitor(&self) -> Vec<PromptMessage> {
        vec![
            PromptMessage::new_text(
                PromptMessageRole::Assistant,
                "You are a real-time departure monitor assistant for Dresden's public transportation system (DVB). \
                 Your primary focus is to quickly provide departure information from specific stations. \
                 When a user asks about departures, use find_stations to locate the stop, then use monitor_departures to show real-time information. \
                 Always present departures as a Markdown table with columns: Line, Destination, Departure (in min/time), Platform/Bay, and Delay/Status if available. \
                 Highlight any delays or disruptions. Be concise and fast—users at a stop need quick answers. \
                 If the user doesn't specify a station, ask them which stop they're interested in.",
            ),
            PromptMessage::new_text(
                PromptMessageRole::User,
                "When is the next tram from Postplatz?",
            ),
            PromptMessage::new_text(
                PromptMessageRole::Assistant,
                "Let me check the real-time departures at Postplatz for you. Here are the next trams:\n\n\
| Line | Destination | Departure | Platform | Status |\n\
|------|-------------|-----------|----------|--------|\n\
| 1    | Prohlis     | 2 min     | 2        | On time |\n\
| 2    | Gorbitz     | 5 min     | 1        | +3 min delay |\n\
| 4    | Weinböhla   | 7 min     | 3        | On time |\n\n\
If you want to see departures for a different line or direction, just let me know!",
            ),
        ]
    }

    /// Trip tracker for following a specific trip in real-time
    #[prompt(
        name = "trip-tracker",
        description = "Track a specific trip in real-time to see current location, delays, and connection status"
    )]
    async fn trip_tracker(&self) -> Vec<PromptMessage> {
        vec![
            PromptMessage::new_text(
                PromptMessageRole::Assistant,
                "You are a trip tracking assistant for Dresden's public transportation system (DVB). \
                 Your role is to help users track specific trips in real-time using trip IDs.\n\n\
                 CRITICAL: Trip tracking REQUIRES a trip ID. Trip IDs are obtained from route planning results \
                 (get_route_details tool). Each connection in a route has a unique trip ID that identifies that \
                 specific vehicle's journey.\n\n\
                 WORKFLOW:\n\
                 1. When user asks to track a trip, identify the trip ID from the previous route planning\n\
                 2. ALWAYS use the get_trip_details tool with the trip ID to fetch real-time data\n\
                 3. Store the trip ID in your conversation context for future updates\n\
                 4. When user asks for updates ('Where is my tram?'), use get_trip_details again with the same trip ID\n\n\
                 The get_trip_details tool provides:\n\
                 - Real-time stop sequence and timing\n\
                 - Current vehicle location (which stops are passed/upcoming)\n\
                 - Delay information\n\
                 - Platform/track details\n\n\
                 Display format:\n\
                 - Current status (on time or delayed)\n\
                 - Stops already passed (with checkmarks ✓)\n\
                 - Next upcoming stop (with arrow →)\n\
                 - Future stops on the route\n\
                 - Connection security if they have transfers\n\n\
                 REMEMBER: Without a trip ID, you cannot track a trip. Always keep the trip ID in context \
                 throughout the conversation so you can provide updates when asked.",
            ),
            PromptMessage::new_text(
                PromptMessageRole::User,
                "Track my trip on tram 3 that leaves at 14:12 from Albertplatz",
            ),
            PromptMessage::new_text(
                PromptMessageRole::Assistant,
                "I'll track your trip on tram 3 departing at 14:12 from Albertplatz. \
Let me use get_trip_details with the trip ID from your route.\n\n\
[Using get_trip_details with trip_id: \"voe:11003: :R:j24\"]\n\n\
**Current Status:** On time\n\n\
**Trip Progress:**\n\
✓ Albertplatz (14:12) - Departed\n\
→ Carolaplatz (14:15) - Next stop, arriving in 2 minutes\n\
  Pirnaischer Platz (14:17)\n\
  Hauptbahnhof (14:20)\n\
  Walpurgisstraße (14:23)\n\
  Münchner Platz (14:25)\n\n\
I'm tracking trip ID \"voe:11003: :R:j24\". Just ask me 'Where is my tram?' anytime for an update, \
and I'll use get_trip_details to fetch the latest real-time information!",
            ),
        ]
    }
}

#[tool_router]
impl DVBServer {
    #[tool(
        // name = "get_user_context",
        description = "IMPORTANT: Call this at the start of conversations to get user's saved origin, current location, destination, and preferences. Returns all context in one call to avoid redundant questions."
    )]
    async fn get_user_context(&self) -> Result<CallToolResult, McpError> {
        let origin = self.user_origin.lock().await.clone();
        let location = self.user_location.lock().await.clone();
        let destination = self.user_destination.lock().await.clone();

        let context = UserContext::new(origin, location, destination);

        Ok(success_json(&context))
    }

    #[tool(
        description = "Ask the user's journey origin/starting point. This is where the user's journey begins."
    )]
    async fn elicit_origin(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let current_origin = if let Some(origin) = self.user_origin.lock().await.clone() {
            origin
        } else {
            match context
                .peer
                .elicit::<args::OriginInfo>(
                    "Please provide where you are starting from".to_string(),
                )
                .await
            {
                Ok(Some(user_info)) => {
                    let origin = user_info.origin.clone();
                    *self.user_origin.lock().await = Some(origin.clone());
                    origin
                }
                Ok(None) => "Hauptbahnhof Dresden".to_string(), // Never happen if client checks schema
                Err(_) => return Ok(error_text("unable to determine origin")),
            }
        };

        Ok(success_text(format!("Starting from {}!", current_origin)))
    }

    #[tool(
        description = "Ask the user's current location. This is where the user is right now, which may differ from their journey origin."
    )]
    async fn elicit_location(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let current_location = if let Some(location) = self.user_location.lock().await.clone() {
            location
        } else {
            match context
                .peer
                .elicit::<args::LocationInfo>("Please provide where you are right now".to_string())
                .await
            {
                Ok(Some(user_info)) => {
                    let location = user_info.location.clone();
                    *self.user_location.lock().await = Some(location.clone());
                    location
                }
                Ok(None) => "Hauptbahnhof Dresden".to_string(), // Never happen if client checks schema
                Err(_) => return Ok(error_text("unable to determine location")),
            }
        };

        Ok(success_text(format!("Currently at {}!", current_location)))
    }

    #[tool(
        description = "Ask the user's destination. This is to be used as the end point for trips."
    )]
    async fn elicit_destination(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let current_destination = if let Some(destination) =
            self.user_destination.lock().await.clone()
        {
            destination
        } else {
            match context
                .peer
                .elicit::<args::DestinationInfo>("Please provide where you want to go.".to_string())
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

    #[tool(
        description = "Set the user's journey origin/starting point directly when provided in conversation. Use this when the user tells you where they're starting from (e.g., 'I'm at Hauptbahnhof'). For interactive prompting, use elicit_origin instead."
    )]
    async fn set_origin(
        &self,
        Parameters(args::OriginInfo { origin }): Parameters<args::OriginInfo>,
    ) -> Result<CallToolResult, McpError> {
        *self.user_origin.lock().await = Some(origin.clone());
        Ok(success_text(format!("Origin set to: {}", origin)))
    }

    #[tool(
        description = "Set the user's current location directly when provided in conversation. Use this when the user tells you where they are right now (e.g., 'I'm currently at Altmarkt'). For interactive prompting, use elicit_location instead."
    )]
    async fn set_location(
        &self,
        Parameters(args::LocationInfo { location }): Parameters<args::LocationInfo>,
    ) -> Result<CallToolResult, McpError> {
        *self.user_location.lock().await = Some(location.clone());
        Ok(success_text(format!(
            "Current location set to: {}",
            location
        )))
    }

    #[tool(
        description = "Set the user's destination directly when provided in conversation. Use this when the user tells you where they want to go (e.g., 'I need to go to the airport'). For interactive prompting, use elicit_destination instead."
    )]
    async fn set_destination(
        &self,
        Parameters(args::DestinationInfo { destination }): Parameters<args::DestinationInfo>,
    ) -> Result<CallToolResult, McpError> {
        *self.user_destination.lock().await = Some(destination.clone());
        Ok(success_text(format!("Destination set to: {}", destination)))
    }

    #[tool(description = "Returns the current local time in ISO8601 (RFC3339) format.")]
    async fn now(&self) -> Result<CallToolResult, McpError> {
        let now = chrono::Local::now().to_rfc3339();
        Ok(success_text(now))
    }

    #[tool(description = "Returns a link to OpenStreetMap for the given coordinates.")]
    fn osm_link(
        &self,
        Parameters(args::OsmLinkRequest {
            latitude,
            longitude,
        }): Parameters<args::OsmLinkRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Basic validation
        if !((-90.0..=90.0).contains(&latitude) && (-180.0..=180.0).contains(&longitude)) {
            return Ok(error_text("Invalid latitude or longitude"));
        }
        let link = format!(
            "https://www.openstreetmap.org/?mlat={}&mlon={}&zoom=17",
            latitude, longitude
        );
        Ok(success_json(&OsmLinkResponse { link }))
    }

    #[tool(
        description = "Clear the stored origin, location, and destination for journey planning; will be requested again on next search."
    )]
    async fn reset_context(&self) -> Result<CallToolResult, McpError> {
        *self.user_origin.lock().await = None;
        *self.user_location.lock().await = None;
        *self.user_destination.lock().await = None;
        Ok(success_text(
            "User origin, location, and destination reset. They will be requested again when needed.",
        ))
    }

    #[tool(
        description = "Search for tram stops, bus stops, or train stations in Dresden using a partial or approximate name."
    )]
    async fn find_stations(
        &self,
        Parameters(args::FindStationRequest { rough_stop_name }): Parameters<
            args::FindStationRequest,
        >,
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
        Parameters(args::FindNearbyStationRequest { rough_stop_name }): Parameters<
            args::FindNearbyStationRequest,
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
        description = r#"Search for points of interest (POIs) in Dresden using a partial or approximate name.
        Use this if you only get a rough description of a location or of where the user is to determine their location."#
    )]
    async fn find_pois(
        &self,
        Parameters(args::FindPoiRequest { rough_poi_name }): Parameters<args::FindPoiRequest>,
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
    #[tool(
        description = "Get upcoming departures from a specified stop or station in Dresden. Optionally filter by mode of transport and limit the number of results."
    )]
    async fn monitor_departures(
        &self,
        Parameters(args::MonitorDeparturesRequest {
            stop_name,
            stop_id,
            mot,
            limit,
        }): Parameters<args::MonitorDeparturesRequest>,
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
            mot: mot_filter.as_deref(),
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
        Parameters(args::LinesRequest { point_id }): Parameters<args::LinesRequest>,
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

    #[tool(
        description = "Get detailed information for a specific trip, including all stops and times. Time must be an ISO8601 string."
    )]
    async fn get_trip_details(
        &self,
        Parameters(args::TripDetailsRequest {
            trip_id,
            time,
            stop_id,
            mapdata,
        }): Parameters<args::TripDetailsRequest>,
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

    #[tool(
        description = "Query possible routes between two stops in Dresden. Returns possible trips, departure and arrival info, etc."
    )]
    async fn get_route_details(
        &self,
        Parameters(args::RouteRequest {
            origin,
            destination,
            time,
            isarrivaltime,
            shorttermchanges,
            format,
            via,
        }): Parameters<args::RouteRequest>,
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
        Parameters(args::FindStationRequest { rough_stop_name }): Parameters<
            args::FindStationRequest,
        >,
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
    let found_origin = find_stops(query).await?;
    let Point { id, .. } = found_origin
        .points
        .first()
        .ok_or_else(|| anyhow!("empty response"))?;
    Ok(id.to_owned())
}

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
        _request: Option<PaginatedRequestParam>,
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
        ReadResourceRequestParam { uri }: ReadResourceRequestParam,
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
            _ => Err(McpError::resource_not_found(
                "Resource not found",
                Some(serde_json::json!({ "uri": uri })),
            )),
        }
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        // Future: Add templates for dvb://station/{id}/departures, etc.
        Ok(ListResourceTemplatesResult {
            next_cursor: None,
            resource_templates: Vec::new(),
            meta: None,
        })
    }
}
