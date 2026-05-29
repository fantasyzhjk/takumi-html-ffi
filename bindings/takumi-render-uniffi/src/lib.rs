mod api;
mod cache;
mod error;
mod markdown;
mod renderer;
mod template;

pub use api::{
    RenderInput, ImageFormat, InlineTemplateInput, MeasuredLayout, RenderRequest, RenderSize,
    TemplateRequest, RenderedImage, TemplateContentKind, TemplateInput,
};
pub use error::{RendererError, Result};
pub use renderer::Renderer;
pub use template::TemplateEngine;

uniffi::setup_scaffolding!();
