use takumi::prelude::{OutputFormat, Quality};

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ImageFormat {
    Png,
    WebP,
    Jpeg,
}

impl ImageFormat {
    pub(crate) fn into_output_format(self, quality: Option<u8>) -> OutputFormat {
        match self {
            Self::Png => OutputFormat::Png,
            Self::WebP => match quality {
                Some(value) if value < 100 => OutputFormat::WebP {
                    quality: Quality::new(value),
                },
                _ => OutputFormat::WebPLossless,
            },
            Self::Jpeg => OutputFormat::Jpeg {
                quality: Quality::new(quality.unwrap_or(75)),
            },
        }
    }

    pub(crate) fn content_type(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::WebP => "image/webp",
            Self::Jpeg => "image/jpeg",
        }
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

#[cfg(test)]
mod tests {
    use takumi::prelude::OutputFormat;

    use super::ImageFormat;

    #[test]
    fn jpeg_quality_defaults_to_75_and_clamps_to_100() {
        for (requested, expected) in [(None, 75), (Some(42), 42), (Some(255), 100)] {
            let OutputFormat::Jpeg { quality } = ImageFormat::Jpeg.into_output_format(requested)
            else {
                panic!("JPEG must map to OutputFormat::Jpeg");
            };
            assert_eq!(quality.get(), expected);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn webp_quality_preserves_v1_lossy_and_lossless_semantics() {
        let OutputFormat::WebP { quality } = ImageFormat::WebP.into_output_format(Some(64)) else {
            panic!("quality below 100 must select lossy WebP");
        };
        assert_eq!(quality.get(), 64);

        assert!(matches!(
            ImageFormat::WebP.into_output_format(None),
            OutputFormat::WebPLossless
        ));
        assert!(matches!(
            ImageFormat::WebP.into_output_format(Some(100)),
            OutputFormat::WebPLossless
        ));
        assert!(matches!(
            ImageFormat::WebP.into_output_format(Some(255)),
            OutputFormat::WebPLossless
        ));
    }

    #[test]
    fn png_ignores_quality() {
        assert!(matches!(
            ImageFormat::Png.into_output_format(Some(1)),
            OutputFormat::Png
        ));
    }
}
