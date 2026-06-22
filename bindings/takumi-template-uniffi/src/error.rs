use std::{io, string::FromUtf8Error};

pub type Result<T> = std::result::Result<T, TemplateError>;

#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum TemplateError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("template not found: {0}")]
    TemplateNotFound(String),
    #[error("failed to parse JSON context: {0}")]
    Json(String),
    #[error("failed to render template: {0}")]
    Template(String),
    #[error("I/O error: {0}")]
    Io(String),
}

impl TemplateError {
    pub(crate) fn invalid_request(message: impl Into<String>) -> Self {
        Self::InvalidRequest(message.into())
    }

    pub(crate) fn template_not_found(message: impl Into<String>) -> Self {
        Self::TemplateNotFound(message.into())
    }
}

impl From<io::Error> for TemplateError {
    fn from(value: io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

impl From<serde_json::Error> for TemplateError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value.to_string())
    }
}

impl From<minijinja::Error> for TemplateError {
    fn from(value: minijinja::Error) -> Self {
        Self::Template(value.to_string())
    }
}

impl From<FromUtf8Error> for TemplateError {
    fn from(value: FromUtf8Error) -> Self {
        Self::Template(value.to_string())
    }
}

pub(crate) fn state_lock_error(name: &str) -> TemplateError {
    TemplateError::Template(format!("state lock `{name}` was poisoned"))
}
