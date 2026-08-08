/// Event type identifiers for the messaging system.
pub struct EventTypes;

impl EventTypes {
    pub const API_HIT: &'static str = "API_HIT";
}

/// Exchange names for RabbitMQ routing.
pub struct Exchanges;

impl Exchanges {
    pub const SERVER_EVENTS: &'static str = "server_events";
}
