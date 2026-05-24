mod api;
mod cache;
mod error;
mod renderer;
mod template;

pub use api::{ImageFormat, RenderRequest, RenderSize, RenderedImage};
pub use error::{RendererError, Result};
pub use renderer::Renderer;

uniffi::setup_scaffolding!();
