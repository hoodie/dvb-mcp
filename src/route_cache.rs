use chrono::{DateTime, Local};
use std::collections::HashMap;

// Import shared types from dvb-rs
use dvb::route::{PartialRoute, Route};

/// Type aliases for clarity
pub type RouteKey = String; // e.g., "direction|name|product_name|departure_time"
pub type PartialRouteKey = String; // same pattern

/// Short summary of a route for display and lookup
#[derive(Debug, Clone)]
pub struct RouteSummary {
    pub key: RouteKey,
    pub summary: String,
    pub departure_time: DateTime<Local>,
}

impl RouteSummary {
    pub fn is_future(&self, now: DateTime<Local>) -> bool {
        self.departure_time > now
    }
}

/// Cache for last found routes (global, not per-session)
#[derive(Default)]
pub struct RouteCache {
    pub routes: HashMap<RouteKey, Route>,
    pub partial_routes: HashMap<PartialRouteKey, PartialRoute>,
    pub route_to_partial: HashMap<RouteKey, Vec<PartialRouteKey>>,
}

pub fn make_composite_key(
    direction: &str,
    name: &str,
    product_name: &str,
    departure_time: DateTime<Local>,
) -> String {
    format!(
        "{}|{}|{}|{}",
        direction,
        name,
        product_name,
        departure_time.format("%Y%m%dT%H%M%S")
    )
}

impl RouteCache {
    /// Store new route results, replacing the cache
    pub fn store_routes(&mut self, routes: Vec<Route>) {
        self.routes.clear();
        self.partial_routes.clear();
        self.route_to_partial.clear();

        for route in routes {
            // Use the first MotChain as the main leg for the route key
            if let Some(mot_chain) = route.mot_chain.as_ref().and_then(|mc| mc.get(0)) {
                let (_, _, departure_time) = extract_route_info(&route);
                let key = make_composite_key(
                    mot_chain.direction.as_deref().unwrap_or(""),
                    mot_chain.name.as_deref().unwrap_or(""),
                    mot_chain.product_name.as_deref().unwrap_or(""),
                    departure_time,
                );
                self.routes.insert(key.clone(), route.clone());

                // For each partial route, build a key and store
                if let Some(partials) = route.partial_routes.as_ref() {
                    let mut partial_keys = Vec::new();
                    for partial in partials {
                        if let Some(mot) = partial.mot.as_ref() {
                            // For partials, try to extract departure_time from the first stop
                            let partial_departure_time = partial
                                .regular_stops
                                .as_ref()
                                .and_then(|stops| stops.get(0))
                                .and_then(|stop| stop.departure_time.clone())
                                .map(|dvb_time| {
                                    let dt_fixed: chrono::DateTime<chrono::FixedOffset> =
                                        dvb_time.to_datetime();
                                    dt_fixed.with_timezone(&Local)
                                })
                                .unwrap_or_else(|| Local::now());
                            let pkey = make_composite_key(
                                mot.direction.as_deref().unwrap_or(""),
                                mot.name.as_deref().unwrap_or(""),
                                mot.product_name.as_deref().unwrap_or(""),
                                partial_departure_time,
                            );
                            self.partial_routes.insert(pkey.clone(), partial.clone());
                            partial_keys.push(pkey);
                        }
                    }
                    self.route_to_partial.insert(key, partial_keys);
                }
            }
        }
    }

    /// Query all routes
    pub fn get_routes(&self) -> &HashMap<RouteKey, Route> {
        &self.routes
    }

    /// Query a specific route by key
    pub fn get_route(&self, route_key: &RouteKey) -> Option<&Route> {
        self.routes.get(route_key)
    }

    /// Query a specific partial route by key
    pub fn get_partial_route(&self, partial_key: &PartialRouteKey) -> Option<&PartialRoute> {
        self.partial_routes.get(partial_key)
    }

    /// Query all partial route keys for a route
    pub fn get_partial_keys_for_route(
        &self,
        route_key: &RouteKey,
    ) -> Option<&Vec<PartialRouteKey>> {
        self.route_to_partial.get(route_key)
    }

    /// Returns a list of short route descriptions with future/missed flag
    pub fn get_route_summaries(&self, now: DateTime<Local>) -> Vec<RouteSummary> {
        let mut summaries = Vec::new();
        for (key, route) in &self.routes {
            let (origin, destination, departure_time) = extract_route_info(route);
            let summary = format!(
                "{} von {} nach {} um {}",
                route
                    .mot_chain
                    .as_ref()
                    .and_then(|mc| mc.get(0))
                    .and_then(|m| m.product_name.clone())
                    .unwrap_or_else(|| "Unbekannt".to_string()),
                origin,
                destination,
                departure_time.format("%H:%M")
            );
            summaries.push(RouteSummary {
                key: key.clone(),
                summary,
                departure_time,
            });
        }
        summaries
    }

    /// Returns a route by its summary string (if unique)
    pub fn get_route_by_summary(&self, summary: &str, now: DateTime<Local>) -> Option<&Route> {
        let summaries = self.get_route_summaries(now);
        summaries
            .iter()
            .find(|s| s.summary == summary)
            .and_then(|s| self.get_route(&s.key))
    }
}

/// Helper function to extract origin, destination, and departure time from a Route
fn extract_route_info(route: &Route) -> (String, String, DateTime<Local>) {
    // Try to extract from partial_routes[0].regular_stops[0] and last stop, and departure_time
    let origin = route
        .partial_routes
        .as_ref()
        .and_then(|prs| prs.get(0))
        .and_then(|pr| pr.regular_stops.as_ref()?.get(0))
        .and_then(|stop| stop.name.clone())
        .unwrap_or_else(|| "Unbekannt".to_string());

    let destination = route
        .partial_routes
        .as_ref()
        .and_then(|prs| prs.last())
        .and_then(|pr| pr.regular_stops.as_ref()?.last())
        .and_then(|stop| stop.name.clone())
        .unwrap_or_else(|| "Unbekannt".to_string());

    let departure_time = route
        .partial_routes
        .as_ref()
        .and_then(|prs| prs.get(0))
        .and_then(|pr| pr.regular_stops.as_ref()?.get(0))
        .and_then(|stop| stop.departure_time)
        .map(|dvb_time| dvb_time.to_datetime()
        .unwrap_or_else(|| Local::now().into()); // fallback to now if missing

    (origin, destination, departure_time)
}
