//! Argument types for MCP tools

use chrono::{DateTime, FixedOffset};
use dvb::point::Point;
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
    pub stop_name: Option<String>,
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
pub struct DVBPointCoords {
    pub latitude: i64,
    pub longitude: i64,
}

impl From<Point> for DVBPointCoords {
    fn from(Point { coords, .. }: Point) -> Self {
        Self {
            latitude: coords.0,
            longitude: coords.1,
        }
    }
}
