use takumi::rendering::ImageOutputFormat;

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ImageFormat {
    Png,
    WebP,
    Jpeg
}

impl ImageFormat {
    pub(crate) fn into_output_format(self) -> ImageOutputFormat {
        match self {
            Self::Png => ImageOutputFormat::Png,
            Self::WebP => ImageOutputFormat::WebP,
            Self::Jpeg => ImageOutputFormat::Jpeg,
        }
    }

    pub(crate) fn content_type(self) -> &'static str {
        self.into_output_format().content_type()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RenderSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RenderInput {
    pub source_kind: RenderSourceKind,
    pub content_kind: RenderContentKind,
    pub value: String,
    pub logical_name: Option<String>,
    pub base_path: Option<String>,
    pub search_paths: Option<Vec<String>>,
    pub syntax_theme: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum RenderSourceKind {
    Inline,
    File,
    Registered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum RenderContentKind {
    Html,
    Markdown,
    JinjaHtml,
    JinjaMarkdown,
}

impl RenderContentKind {
    pub(crate) fn requires_jinja(self) -> bool {
        matches!(self, Self::JinjaHtml | Self::JinjaMarkdown)
    }

    pub(crate) fn requires_markdown(self) -> bool {
        matches!(self, Self::Markdown | Self::JinjaMarkdown)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RenderRequest {
    pub input: RenderInput,
    pub context_json: String,
    pub viewport: RenderSize,
    pub format: ImageFormat,
    pub quality: Option<u8>,
    pub load_linked_stylesheets: Option<bool>,
    pub resolve_local_assets: Option<bool>,
    pub normalize_whitespace: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RenderedImage {
    pub bytes: Vec<u8>,
    pub format: ImageFormat,
    pub width: u32,
    pub height: u32,
    pub content_type: Option<String>,
}
