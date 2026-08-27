use std::{io, string::FromUtf8Error};

use takumi::prelude::{Error as TakumiError, FontError};
use takumi_html::HtmlError;

pub type Result<T> = std::result::Result<T, RendererError>;

#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum RendererError {
    #[error("invalid render request: {0}")]
    InvalidRequest(String),
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
}

impl From<io::Error> for RendererError {
    fn from(value: io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

impl From<HtmlError> for RendererError {
    fn from(value: HtmlError) -> Self {
        Self::Html(value.to_string())
    }
}

impl From<TakumiError> for RendererError {
    fn from(value: TakumiError) -> Self {
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
        Self::Render(value.to_string())
    }
}

pub(crate) fn state_lock_error(name: &str) -> RendererError {
    RendererError::Render(format!("state lock `{name}` was poisoned"))
}
