use server_monitoring::domain::updates::{ApiKeyUpdate, UserProfileUpdate};

#[test]
fn user_profile_update_empty_by_default() {
    let update = UserProfileUpdate::default();
    assert!(update.is_empty());
    assert!(update.username.is_none());
    assert!(update.email.is_none());
}

#[test]
fn user_profile_update_not_empty_with_username() {
    let update = UserProfileUpdate {
        username: Some("new_user".to_string()),
        ..Default::default()
    };
    assert!(!update.is_empty());
}

#[test]
fn user_profile_update_not_empty_with_email() {
    let update = UserProfileUpdate {
        email: Some("new@example.com".to_string()),
        ..Default::default()
    };
    assert!(!update.is_empty());
}

#[test]
fn api_key_update_default_is_all_none() {
    let update = ApiKeyUpdate::default();
    assert!(update.name.is_none());
    assert!(update.key_value.is_none());
    assert!(update.is_active.is_none());
    assert!(update.allowed_ips.is_none());
    assert!(update.allowed_origins.is_none());
    assert!(update.can_ingest.is_none());
    assert!(update.can_read_analytics.is_none());
    assert!(update.last_rotated.is_none());
}

#[test]
fn api_key_update_partial_fields() {
    let update = ApiKeyUpdate {
        name: Some("production-key".to_string()),
        is_active: Some(false),
        ..Default::default()
    };
    assert_eq!(update.name.as_deref(), Some("production-key"));
    assert_eq!(update.is_active, Some(false));
    assert!(update.key_value.is_none());
}
