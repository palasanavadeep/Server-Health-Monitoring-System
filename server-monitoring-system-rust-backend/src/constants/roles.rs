/// Roles constants mirroring Node.js roles.js exactly.

pub const ROLES: &[&str] = &["super_admin", "client_admin", "client_viewer"];

pub const CLIENT_ROLES: &[&str] = &["client_admin", "client_viewer"];

pub struct ApplicationRoles;

impl ApplicationRoles {
    pub const SUPER_ADMIN: &'static str = "super_admin";
    pub const CLIENT_ADMIN: &'static str = "client_admin";
    pub const CLIENT_VIEWER: &'static str = "client_viewer";
}

pub fn is_valid_client_role(role: &str) -> bool {
    CLIENT_ROLES.contains(&role)
}

pub fn is_valid_role(role: &str) -> bool {
    ROLES.contains(&role)
}
