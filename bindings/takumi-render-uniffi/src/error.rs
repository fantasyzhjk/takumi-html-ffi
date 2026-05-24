use std::{io, string::FromUtf8Error};

use takumi::resources::font::FontError;
use takumi_html::HtmlError;

pub type Result<T> = std::result::Result<T, RendererError>;

#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum RendererError {
    #[error("invalid render request: {0}")]
    InvalidRequest(String),
    #[error("template not found: {0}")]
    TemplateNotFound(String),
    #[error("failed to parse JSON context: {0}")]
    Json(String),
    #[error("failed to render template: {0}")]
    Template(String),
    #[error("failed to convert HTML: {0}")]
    Html(String),
    #[error("failed to load font: {0}")]
    Font(String),
    #[error("failed to render image: {0}")]
    Render(String),
    #[error("failed to encode image: {0}")]
    Encode(String),
    #[error("I/O error: {0}")]
    Io(String),
}

impl RendererError {
    pub(crate) fn invalid_request(message: impl Into<String>) -> Self {
        Self::InvalidRequest(message.into())
    }

    pub(crate) fn template_not_found(message: impl Into<String>) -> Self {
        Self::TemplateNotFound(message.into())
    }

    pub(crate) fn encode(message: impl Into<String>) -> Self {
        Self::Encode(message.into())
    }
}

impl From<io::Error> for RendererError {
    fn from(value: io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

impl From<serde_json::Error> for RendererError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value.to_string())
    }
}

impl From<minijinja::Error> for RendererError {
    fn from(value: minijinja::Error) -> Self {
        Self::Template(value.to_string())
    }
}

impl From<HtmlError> for RendererError {
    fn from(value: HtmlError) -> Self {
        Self::Html(value.to_string())
    }
}

impl From<takumi::Error> for RendererError {
    fn from(value: takumi::Error) -> Self {
        Self::Render(value.to_string())
    }
}

impl From<FontError> for RendererError {
    fn from(value: FontError) -> Self {
        Self::Font(value.to_string())
    }
}

impl From<FromUtf8Error> for RendererError {
    fn from(value: FromUtf8Error) -> Self {
        Self::Template(value.to_string())
    }
}
