use anyhow::{Result, anyhow};
use rmcp::{
    ErrorData as McpError,
    handler::server::{
        router::{prompt::PromptRouter, tool::ToolRouter},
        wrapper::Parameters,
    },
    model::*,
    prompt, prompt_router, serde_json,
    service::{RequestContext, RoleServer},
    tool, tool_router,
};
use tokio::sync::Mutex;

use dvb::{find_stops, point::Point};
use std::sync::Arc;

use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod args;
mod osm_links;
mod server_handle;
mod usercontext;

use crate::server::{args::DVBPointCoords, osm_links::OsmCoords, usercontext::UserContext};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct OsmLinkResponse {
    pub link: String,
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

async fn lookup_stop_id(query: &str) -> anyhow::Result<String> {
    let found_origin = find_stops(query).await?;
    let Point { id, .. } = found_origin
        .points
        .first()
        .ok_or_else(|| anyhow!("empty response"))?;
    Ok(id.to_owned())
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
        Parameters(dvb_coords): Parameters<DVBPointCoords>,
    ) -> Result<CallToolResult, McpError> {
        match OsmCoords::try_from(dvb_coords) {
            Ok(osm) => Ok(success_json(&OsmLinkResponse { link: osm.url() })),
            Err(msg) => Ok(error_text(msg.to_string())),
        }
    }

    /// Search for POIs by name using the VVO PointFinder API and return links to OpenStreetMap.
    #[tool(description = "Search for POIs by name and get OpenStreetMap links.")]
    async fn osm_links_from_query(
        &self,
        Parameters(args::FindPoiRequest { rough_poi_name }): Parameters<args::FindPoiRequest>,
    ) -> Result<CallToolResult, McpError> {
        let points = match dvb::find_pois(&rough_poi_name).await {
            Ok(response) => response.into_inner().points,
            Err(error) => {
                return Ok(error_text(format!(
                    "failed to find POI {rough_poi_name:?}: {error}"
                )));
            }
        };

        let mut results: Vec<String> = Vec::new();
        for point in points {
            let request = match OsmCoords::try_from(point.clone()) {
                Ok(request) => request,
                Err(error) => {
                    results.push(error.to_string());
                    continue;
                }
            };

            results.push(request.url())
        }

        Ok(success_json(&serde_json::json!({
            "results": results
        })))
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
            mobility_settings: None,
            standard_settings: None,
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
    async fn lookup_stop_id(
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

impl DVBServer {
    /// List all available tools
    pub fn list_tools(&self) {
        println!("Available Tools:");
        println!("================\n");

        for tool in self.tool_router.list_all() {
            println!("  • {}", tool.name);
            if let Some(description) = &tool.description {
                println!("    {}\n", description);
            } else {
                println!();
            }
        }
    }

    /// List all available prompts
    pub fn list_prompts(&self) {
        println!("Available Prompts:");
        println!("==================\n");

        for prompt in self.prompt_router.list_all() {
            println!("  • {}", prompt.name);
            if let Some(description) = &prompt.description {
                println!("    {}\n", description);
            } else {
                println!();
            }
        }
    }

    /// List context keys
    pub fn list_context_keys(&self) {
        println!("Context Keys:");
        println!("=============\n");

        // Generate schema from UserContext and serialize to JSON
        let schema = rmcp::schemars::schema_for!(UserContext);
        let schema_json = serde_json::to_value(&schema).unwrap();

        if let Some(properties) = schema_json["properties"].as_object() {
            for (name, prop) in properties {
                println!("  • {}", name);
                if let Some(description) = prop["description"].as_str() {
                    println!("    {}\n", description);
                } else {
                    println!();
                }
            }
        }
    }
}
