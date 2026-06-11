mod api;
mod cache;
mod error;
mod markdown;
mod renderer;
mod template;

pub use api::{
    ImageFormat, InlineTemplateInput, MeasuredLayout, RenderInput, RenderRequest, RenderSize,
    RenderedImage, TemplateContentKind, TemplateInput, TemplateRequest,
};
pub use error::{RendererError, Result};
pub use renderer::Renderer;
pub use template::TemplateEngine;

uniffi::setup_scaffolding!();
