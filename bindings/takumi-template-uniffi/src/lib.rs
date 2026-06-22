mod api;
mod cache;
mod error;
mod markdown;
mod template;

pub use api::{InlineTemplateInput, TemplateContentKind, TemplateInput, TemplateRequest};
pub use error::{TemplateError, Result};
pub use template::TemplateEngine;

uniffi::setup_scaffolding!();
