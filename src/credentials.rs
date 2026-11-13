/// Normalize and validate email addresses consistently across HTTP and CLI entry points.
pub fn normalize_email(email: &str) -> Result<String, &'static str> {
    let email = email.trim().to_lowercase();
    let valid = email.split_once('@').is_some_and(|(local, domain)| {
        !local.is_empty()
            && domain.contains('.')
            && !domain.starts_with('.')
            && !domain.ends_with('.')
    });
    if !valid
        || email.matches('@').count() != 1
        || email.len() > 254
        || email.chars().any(char::is_whitespace)
    {
        return Err("A valid email is required");
    }
    Ok(email)
}

pub fn validate_password(password: &str) -> Result<(), &'static str> {
    if !(12..=128).contains(&password.len()) {
        return Err("Password must be between 12 and 128 bytes");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_emails_that_cannot_be_used_to_log_in() {
        for email in [
            "demo@localhost",
            "@example.com",
            "a@@example.com",
            "a b@example.com",
            "a@.example.com",
        ] {
            assert!(normalize_email(email).is_err(), "accepted {email}");
        }
        assert_eq!(
            normalize_email("  Demo@Example.COM  "),
            Ok("demo@example.com".into())
        );
    }

    #[test]
    fn validates_password_byte_boundaries() {
        for length in [0, 11, 129] {
            assert!(validate_password(&"a".repeat(length)).is_err());
        }
        for length in [12, 128] {
            assert!(validate_password(&"a".repeat(length)).is_ok());
        }
        assert!(validate_password(&"é".repeat(65)).is_err());
    }
}
