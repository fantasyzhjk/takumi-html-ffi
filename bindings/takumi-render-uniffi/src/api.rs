use takumi::rendering::ImageOutputFormat;

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ImageFormat {
    Png,
    WebP,
    Jpeg,
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

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct RenderSize {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub device_pixel_ratio: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct InlineTemplateInput {
    pub source: String,
    pub logical_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum TemplateInput {
    Inline(InlineTemplateInput),
    File(String),
    Registered(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum TemplateContentKind {
    Markdown,
    JinjaHtml,
    JinjaMarkdown,
}

impl TemplateContentKind {
    pub(crate) fn requires_jinja(self) -> bool {
        matches!(self, Self::JinjaHtml | Self::JinjaMarkdown)
    }

    pub(crate) fn requires_markdown(self) -> bool {
        matches!(self, Self::Markdown | Self::JinjaMarkdown)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TemplateRequest {
    pub input: TemplateInput,
    pub context_json: Option<String>,
    pub content_kind: TemplateContentKind,
    pub syntax_theme: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum RenderInput {
    Inline(String),
    File(String),
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct RenderRequest {
    pub input: RenderInput,
    pub viewport: RenderSize,
    pub format: ImageFormat,
    pub quality: Option<u8>,
    pub load_linked_stylesheets: Option<bool>,
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

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MeasuredLayout {
    pub width: u32,
    pub height: u32,
}
