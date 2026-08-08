use serde::{Deserialize, Serialize};
use std::fmt;

/// Application roles as a strict enum.
///
/// Provides type-safe role checking throughout the codebase. Serializes to
/// snake_case strings for MongoDB/JSON compatibility (e.g. `"super_admin"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    #[serde(rename = "super_admin")]
    SuperAdmin,

    #[serde(rename = "client_admin")]
    ClientAdmin,

    #[serde(rename = "client_viewer")]
    ClientViewer,
}

impl Role {
    /// All valid roles in the system.
    pub const ALL: &'static [Role] = &[Role::SuperAdmin, Role::ClientAdmin, Role::ClientViewer];

    /// Roles that can be assigned to client users (excludes SuperAdmin).
    pub const CLIENT_ROLES: &'static [Role] = &[Role::ClientAdmin, Role::ClientViewer];

    /// Check if this role is a valid client role.
    pub fn is_client_role(&self) -> bool {
        Self::CLIENT_ROLES.contains(self)
    }

    /// Check if this is a super admin role.
    pub fn is_super_admin(&self) -> bool {
        *self == Role::SuperAdmin
    }

    /// Returns the string representation used in storage/API.
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::SuperAdmin => "super_admin",
            Role::ClientAdmin => "client_admin",
            Role::ClientViewer => "client_viewer",
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for Role {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "super_admin" => Ok(Role::SuperAdmin),
            "client_admin" => Ok(Role::ClientAdmin),
            "client_viewer" => Ok(Role::ClientViewer),
            _ => Err(format!("Invalid role: '{}'", s)),
        }
    }
}

/// Default role for new users.
impl Default for Role {
    fn default() -> Self {
        Role::ClientViewer
    }
}

/// Check if a string represents a valid client role.
pub fn is_valid_client_role(role: &str) -> bool {
    role.parse::<Role>()
        .map(|r| r.is_client_role())
        .unwrap_or(false)
}

/// Check if a string represents any valid role.
pub fn is_valid_role(role: &str) -> bool {
    role.parse::<Role>().is_ok()
}
