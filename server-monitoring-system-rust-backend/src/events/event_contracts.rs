/// Event type constants mirroring Node.js eventContracts.js.
pub struct EventTypes;

impl EventTypes {
    pub const API_HIT: &'static str = "API_HIT";
}

/// Exchange config mirroring Node.js eventContracts.js.
pub struct Exchanges;

impl Exchanges {
    pub const SERVER_EVENTS: &'static str = "server_events";
}
