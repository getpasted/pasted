use std::fmt;

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationError {
    pub code: &'static str,
    pub message: String,
}

impl ApplicationError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new("invalid_input", message)
    }

    pub fn persistence(error: impl fmt::Display) -> Self {
        Self::new("persistence_failed", error.to_string())
    }
}

impl fmt::Display for ApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ApplicationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialized_errors_have_a_stable_cross_surface_shape() {
        assert_eq!(
            serde_json::to_value(ApplicationError::invalid("Bad value")).unwrap(),
            serde_json::json!({ "code": "invalid_input", "message": "Bad value" })
        );
    }
}
