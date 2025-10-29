use std::collections::HashMap;

// Import shared types from dvb-rs
use dvb::route::{Mot, MotChain, PartialRoute, Route};

/// Type aliases for clarity
pub type SessionId = String;
pub struct RouteKey(String); // e.g., "direction|name|product_name"
pub type PartialRouteKey = String; // same pattern

/// Cache for a user's last found routes
#[derive(Default)]
pub struct RouteCache {
    pub routes: HashMap<RouteKey, Route>,
    pub partial_routes: HashMap<PartialRouteKey, PartialRoute>,
    pub route_to_partial: HashMap<RouteKey, Vec<PartialRouteKey>>,
}

/// Top-level cache: session/user -> RouteCache
#[derive(Default)]
pub struct LastFoundRoutes {
    pub sessions: HashMap<SessionId, RouteCache>,
}

/// Utility to build a composite key from direction, name, and product_name
pub fn make_composite_key(direction: &str, name: &str, product_name: &str) -> String {
    format!("{}|{}|{}", direction, name, product_name)
}

impl LastFoundRoutes {
    /// Store new route results for a session
    pub fn store_routes(&mut self, session: SessionId, routes: Vec<Route>) {
        let mut route_cache = RouteCache::default();

        for route in routes {
            // Use the first MotChain as the main leg for the route key
            if let Some(mot_chain) = route.mot_chain.as_ref().and_then(|mc| mc.get(0)) {
                let key = make_composite_key(
                    mot_chain.direction.as_deref().unwrap_or(""),
                    mot_chain.name.as_deref().unwrap_or(""),
                    mot_chain.product_name.as_deref().unwrap_or(""),
                );
                route_cache.routes.insert(key.clone(), route.clone());

                // For each partial route, build a key and store
                if let Some(partials) = route.partial_routes.as_ref() {
                    let mut partial_keys = Vec::new();
                    for partial in partials {
                        if let Some(mot) = partial.mot.as_ref() {
                            let pkey = make_composite_key(
                                mot.direction.as_deref().unwrap_or(""),
                                mot.name.as_deref().unwrap_or(""),
                                mot.product_name.as_deref().unwrap_or(""),
                            );
                            route_cache
                                .partial_routes
                                .insert(pkey.clone(), partial.clone());
                            partial_keys.push(pkey);
                        }
                    }
                    route_cache.route_to_partial.insert(key, partial_keys);
                }
            }
        }
        self.sessions.insert(session, route_cache);
    }

    /// Query all routes for a session
    pub fn get_routes(&self, session: &SessionId) -> Option<&HashMap<RouteKey, Route>> {
        self.sessions.get(session).map(|cache| &cache.routes)
    }

    /// Query a specific route by key
    pub fn get_route(&self, session: &SessionId, route_key: &RouteKey) -> Option<&Route> {
        self.sessions
            .get(session)
            .and_then(|cache| cache.routes.get(route_key))
    }

    /// Query a specific partial route by key
    pub fn get_partial_route(
        &self,
        session: &SessionId,
        partial_key: &PartialRouteKey,
    ) -> Option<&PartialRoute> {
        self.sessions
            .get(session)
            .and_then(|cache| cache.partial_routes.get(partial_key))
    }

    /// Query all partial route keys for a route
    pub fn get_partial_keys_for_route(
        &self,
        session: &SessionId,
        route_key: &RouteKey,
    ) -> Option<&Vec<PartialRouteKey>> {
        self.sessions
            .get(session)
            .and_then(|cache| cache.route_to_partial.get(route_key))
    }
}
