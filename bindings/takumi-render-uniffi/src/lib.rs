mod api;
mod cache;
mod error;
mod markdown;
mod renderer;
mod template;

pub use api::{
	ImageFormat,
	RenderContentKind,
	RenderInput,
	RenderRequest,
	RenderSize,
	RenderSourceKind,
	RenderedImage,
};
pub use error::{RendererError, Result};
pub use renderer::Renderer;

uniffi::setup_scaffolding!();
