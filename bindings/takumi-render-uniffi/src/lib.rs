mod api;
mod cache;
mod error;
mod markdown;
mod renderer;
mod template;

pub use api::{
    HtmlInput, ImageFormat, InlineTemplateInput, MeasuredLayout, RenderHtmlRequest, RenderSize,
    RenderTemplateRequest, RenderedImage, TemplateContentKind, TemplateInput,
};
pub use error::{RendererError, Result};
pub use renderer::Renderer;
pub use template::TemplateEngine;

uniffi::setup_scaffolding!();
