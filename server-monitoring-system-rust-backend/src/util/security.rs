/// Password strength validation.
///
/// Checks passwords against configurable security requirements including
/// length, character class rules, and known weak password patterns.
pub struct SecurityUtils;

/// Result of password validation.
pub struct PasswordValidationResult {
    pub success: bool,
    pub errors: Vec<String>,
}

impl SecurityUtils {
    /// Validates a password against security requirements.
    pub fn validate_password(password: &str) -> PasswordValidationResult {
        let mut errors = Vec::new();

        let min_length: usize = std::env::var("PASSWORD_MIN_LENGTH")
            .unwrap_or_else(|_| "8".to_string())
            .parse()
            .unwrap_or(8);
        let require_uppercase = std::env::var("PASSWORD_REQUIRE_UPPERCASE")
            .unwrap_or_else(|_| "true".to_string())
            == "true";
        let require_lowercase = std::env::var("PASSWORD_REQUIRE_LOWERCASE")
            .unwrap_or_else(|_| "true".to_string())
            == "true";
        let require_numbers = std::env::var("PASSWORD_REQUIRE_NUMBERS")
            .unwrap_or_else(|_| "true".to_string())
            == "true";
        let require_symbols = std::env::var("PASSWORD_REQUIRE_SYMBOLS")
            .unwrap_or_else(|_| "true".to_string())
            == "true";

        if password.is_empty() {
            return PasswordValidationResult {
                success: false,
                errors: vec!["Password is required".to_string()],
            };
        }

        if password.len() < min_length {
            errors.push(format!(
                "Password must be at least {} chars long!",
                min_length
            ));
        }

        if require_uppercase && !password.chars().any(|c| c.is_ascii_uppercase()) {
            errors.push("Password must contain at least one uppercase letter".to_string());
        }

        if require_lowercase && !password.chars().any(|c| c.is_ascii_lowercase()) {
            errors.push("Password must contain at least one lowercase letter".to_string());
        }

        if require_numbers && !password.chars().any(|c| c.is_ascii_digit()) {
            errors.push("Password must contain at least one number".to_string());
        }

        if require_symbols && !password.chars().any(|c| !c.is_alphanumeric()) {
            errors.push("Password must contain at least one special character".to_string());
        }

        let weak_passwords = [
            "password",
            "123456",
            "qwerty",
            "admin",
            "letmein",
            "password123",
            "admin123",
            "12345678",
            "welcome",
        ];

        if weak_passwords.contains(&password.to_lowercase().as_str()) {
            errors.push("Password is too common and easily guessable".to_string());
        }

        PasswordValidationResult {
            success: errors.is_empty(),
            errors,
        }
    }
}
