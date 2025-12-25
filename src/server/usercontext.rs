use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
