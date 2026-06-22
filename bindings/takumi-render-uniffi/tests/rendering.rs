use std::{
    fs,
    path::{Path, PathBuf},
};

use image::{GenericImageView, ImageFormat as DecodedImageFormat};
use serde_json::json;
use takumi_render_uniffi::{
    ImageFormat, InlineTemplateInput, RenderInput, RenderRequest, RenderSize, Renderer,
    TemplateContentKind, TemplateEngine, TemplateInput, TemplateRequest,
};
use tempfile::TempDir;

#[test]
fn render_template_string_supports_nested_json_and_search_paths() {
    let temp = fixture_bundle();
    let template_engine = configured_template_engine(temp.path());
    let renderer = configured_renderer(temp.path());
    let template_source =
        fs::read_to_string(temp.path().join("index.jinja")).expect("read fixture template");
    let html = template_engine
        .render(template_request(
            TemplateInput::Inline(InlineTemplateInput {
                source: template_source,
                logical_name: Some("inline/index.jinja".to_string()),
            }),
            TemplateContentKind::JinjaHtml,
            None,
            Some(sample_context_json()),
        ))
        .expect("render template string");

    let rendered = renderer
        .render(html_request(html, ImageFormat::Png))
        .expect("render html output");

    assert!(!rendered.bytes.is_empty());
    assert_eq!(rendered.format, ImageFormat::Png);
    let decoded = image::load_from_memory_with_format(&rendered.bytes, DecodedImageFormat::Png)
        .expect("decode png bytes");
    assert_eq!(decoded.width(), 64);
    assert_eq!(decoded.height(), 64);
}

#[test]
fn render_inline_html_linear_gradient_background_renders_color_transition() {
    let renderer = Renderer::new();
    let request = RenderRequest {
        input: RenderInput::Inline(
            r#"<style>
      .gradient {
        display: block;
        width: 64px;
        height: 16px;
        background-image: linear-gradient(to right, rgb(255, 0, 0), rgb(0, 0, 255));
      }
    </style>
    <div class="gradient"></div>
"#
            .to_string(),
        ),
        viewport: RenderSize {
            width: Some(64),
            height: Some(16),
            device_pixel_ratio: None,
        },
        format: ImageFormat::Png,
        quality: Some(100),
        load_linked_stylesheets: None,
        normalize_whitespace: None,
    };

    let rendered = renderer
        .render(request)
        .expect("render inline html with linear gradient");

    assert!(!rendered.bytes.is_empty());
    let decoded = image::load_from_memory_with_format(&rendered.bytes, DecodedImageFormat::Png)
        .expect("decode png bytes");
    assert_eq!(decoded.width(), 64);
    assert_eq!(decoded.height(), 16);

    let left = decoded.get_pixel(4, 8).0;
    let center = decoded.get_pixel(32, 8).0;
    let right = decoded.get_pixel(59, 8).0;

    assert!(
        left[0] > left[2],
        "left pixel should remain red-dominant: {left:?}"
    );
    assert!(
        center[0] > 0 && center[2] > 0,
        "center pixel should contain mixed gradient colors: {center:?}"
    );
    assert!(
        right[2] > right[0],
        "right pixel should remain blue-dominant: {right:?}"
    );
}

#[test]
fn render_template_file_to_file_writes_decodable_webp() {
    let temp = fixture_bundle();
    let template_engine = configured_template_engine(temp.path());
    let renderer = configured_renderer(temp.path());
    let output_path = temp.path().join("out/rendered.webp");
    let html = template_engine
        .render(template_request(
            TemplateInput::File("index.jinja".to_string()),
            TemplateContentKind::JinjaHtml,
            None,
            Some(sample_context_json()),
        ))
        .expect("render template file");

    let rendered = renderer
        .render_to_file(
            html_request(html, ImageFormat::WebP),
            output_path.to_string_lossy().into_owned(),
        )
        .expect("render template file to file");

    let output_bytes = fs::read(&output_path).expect("read output file");
    assert_eq!(output_bytes, rendered.bytes);
    let decoded = image::load_from_memory_with_format(&output_bytes, DecodedImageFormat::WebP)
        .expect("decode webp bytes");
    assert_eq!(decoded.width(), 64);
    assert_eq!(decoded.height(), 64);
}

#[test]
fn jpeg_and_webp_encoders_preserve_public_quality_semantics() {
    let renderer = Renderer::new();
    let markup = r#"<style>
html, body { margin: 0; width: 16px; height: 16px; background: rgb(32, 96, 160); }
</style>"#;

    let jpeg = renderer
        .render(inline_render_request(
            markup,
            16,
            16,
            ImageFormat::Jpeg,
            None,
        ))
        .expect("encode default-quality JPEG");
    image::load_from_memory_with_format(&jpeg.bytes, DecodedImageFormat::Jpeg)
        .expect("decode JPEG output");

    let lossy_webp = renderer
        .render(inline_render_request(
            markup,
            16,
            16,
            ImageFormat::WebP,
            Some(64),
        ))
        .expect("encode lossy WebP");
    image::load_from_memory_with_format(&lossy_webp.bytes, DecodedImageFormat::WebP)
        .expect("decode lossy WebP output");
    assert!(contains_webp_chunk(&lossy_webp.bytes, b"VP8 "));
    assert!(!contains_webp_chunk(&lossy_webp.bytes, b"VP8L"));

    for quality in [None, Some(100)] {
        let lossless_webp = renderer
            .render(inline_render_request(
                markup,
                16,
                16,
                ImageFormat::WebP,
                quality,
            ))
            .expect("encode lossless WebP");
        image::load_from_memory_with_format(&lossless_webp.bytes, DecodedImageFormat::WebP)
            .expect("decode lossless WebP output");
        assert!(contains_webp_chunk(&lossless_webp.bytes, b"VP8L"));
        assert!(!contains_webp_chunk(&lossless_webp.bytes, b"VP8 "));
    }
}

#[test]
fn css_default_position_is_static() {
    let image = render_inline_png(
        r#"<style>
html, body { margin: 0; width: 64px; height: 32px; background: white; }
.outer { display: block; position: relative; width: 64px; height: 32px; }
.middle { display: block; margin-left: 24px; width: 16px; height: 16px; background: blue; }
.marker { display: block; position: absolute; left: 0; top: 0; width: 8px; height: 8px; background: red; }
</style>
<div class="outer"><div class="middle"><div class="marker"></div></div></div>"#,
        64,
        32,
    );
    assert_eq!(image.get_pixel(2, 2).0, [255, 0, 0, 255]);
    assert_eq!(image.get_pixel(26, 2).0, [0, 0, 255, 255]);
}

#[test]
fn css_border_and_outline_default_to_medium_width() {
    let image = render_inline_png(
        r#"<style>
html, body { margin: 0; width: 72px; height: 40px; background: white; }
.stage { display: block; position: relative; width: 72px; height: 40px; }
.border { display: block; position: absolute; left: 8px; top: 8px; width: 20px; height: 20px;
          border-style: solid; border-color: red; background: white; }
.outline { display: block; position: absolute; left: 48px; top: 12px; width: 12px; height: 12px;
           outline-style: solid; outline-color: blue; background: white; }
</style>
<div class="stage"><div class="border"></div><div class="outline"></div></div>"#,
        72,
        40,
    );
    assert_eq!(image.get_pixel(8, 16).0, [255, 0, 0, 255]);
    assert_eq!(image.get_pixel(10, 16).0, [255, 0, 0, 255]);
    assert_eq!(image.get_pixel(11, 16).0, [255, 255, 255, 255]);
    assert_eq!(image.get_pixel(45, 16).0, [0, 0, 255, 255]);
    assert_eq!(image.get_pixel(47, 16).0, [0, 0, 255, 255]);
}

#[test]
fn css_negative_scale_reflects_content() {
    let image = render_inline_png(
        r#"<style>
html, body { margin: 0; width: 40px; height: 24px; background: white; }
.stage { display: block; position: relative; width: 40px; height: 24px; }
.shape { display: block; position: absolute; left: 10px; top: 8px; width: 20px; height: 8px;
         transform: scaleX(-1); transform-origin: center;
         background: linear-gradient(to right, red 0 25%, blue 25% 100%); }
</style>
<div class="stage"><div class="shape"></div></div>"#,
        40,
        24,
    );
    assert_eq!(image.get_pixel(12, 12).0, [0, 0, 255, 255]);
    assert_eq!(image.get_pixel(28, 12).0, [255, 0, 0, 255]);
}

#[test]
fn css_line_clamp_shorthand_limits_rendered_lines() {
    let image = render_inline_png(
        r#"<style>
html, body { margin: 0; width: 80px; height: 48px; background: white; }
.clamp { display: block; width: 48px; color: black; font-family: Rubik; font-size: 12px;
         line-height: 14px; line-clamp: 2 "..."; }
</style>
<div class="clamp">first second third fourth fifth sixth seventh eighth ninth</div>"#,
        80,
        48,
    );
    assert!(count_non_white_pixels(&image, 0, 28) > 0);
    assert_eq!(count_non_white_pixels(&image, 30, 48), 0);
}

#[test]
fn svg_image_current_color_is_not_tinted_by_host_color() {
    let image = render_inline_png(
        r#"<style>
html, body { margin: 0; width: 24px; height: 24px; background: white; }
.host { color: red; }
</style>
<div class="host"><svg width="16" height="16" xmlns="http://www.w3.org/2000/svg">
  <rect width="16" height="16" fill="currentColor" />
</svg></div>"#,
        24,
        24,
    );
    assert_eq!(image.get_pixel(8, 8).0, [0, 0, 0, 255]);
}

#[test]
fn registered_templates_render_by_name() {
    let temp = fixture_bundle();
    let template_engine = configured_template_engine(temp.path());
    let renderer = configured_renderer(temp.path());
    let template_source =
        fs::read_to_string(temp.path().join("index.jinja")).expect("read fixture template");
    template_engine
        .add_template("cards/profile.jinja".to_string(), template_source)
        .expect("register template");
    let html = template_engine
        .render(template_request(
            TemplateInput::Registered("cards/profile.jinja".to_string()),
            TemplateContentKind::JinjaHtml,
            None,
            Some(sample_context_json()),
        ))
        .expect("render registered template");

    let rendered = renderer
        .render(html_request(html, ImageFormat::Png))
        .expect("render template by registered name");

    assert!(!rendered.bytes.is_empty());
    assert_eq!(rendered.content_type.as_deref(), Some("image/png"));
}

#[test]
fn font_cache_deduplicates_repeated_font_loads() {
    let renderer = Renderer::new();
    let font_path = font_path();

    renderer
        .add_font_file(font_path.to_string_lossy().into_owned())
        .expect("load font first time");
    renderer
        .add_font_file(font_path.to_string_lossy().into_owned())
        .expect("load font second time");

    assert_eq!(renderer.debug_font_cache_entries(), 1);
}

#[test]
fn render_markdown_file_without_context_json_supports_assets() {
    let temp = fixture_bundle();
    let template_engine = configured_template_engine(temp.path());
    let renderer = configured_renderer(temp.path());
    let html = template_engine
        .render(template_request(
            TemplateInput::File("post.md".to_string()),
            TemplateContentKind::Markdown,
            Some("base16-ocean.dark".to_string()),
            None,
        ))
        .expect("render markdown file");

    let rendered = renderer
        .render(html_request(html, ImageFormat::Png))
        .expect("render markdown html");

    assert!(!rendered.bytes.is_empty());
    assert_eq!(rendered.content_type.as_deref(), Some("image/png"));
    let decoded = image::load_from_memory_with_format(&rendered.bytes, DecodedImageFormat::Png)
        .expect("decode png bytes");
    assert_eq!(decoded.width(), 64);
    assert_eq!(decoded.height(), 64);
}

#[test]
fn render_static_jinja_markdown_file_without_context_json() {
    let temp = fixture_bundle();
    let template_engine = configured_template_engine(temp.path());
    let renderer = configured_renderer(temp.path());
    let html = template_engine
        .render(template_request(
            TemplateInput::File("static-article.jinja.md".to_string()),
            TemplateContentKind::JinjaMarkdown,
            Some("base16-ocean.dark".to_string()),
            None,
        ))
        .expect("render static jinja markdown");

    let rendered = renderer
        .render(html_request(html, ImageFormat::Png))
        .expect("render static jinja markdown html");

    assert!(!rendered.bytes.is_empty());
    let decoded = image::load_from_memory_with_format(&rendered.bytes, DecodedImageFormat::Png)
        .expect("decode png bytes");
    assert_eq!(decoded.width(), 64);
    assert_eq!(decoded.height(), 64);
}

#[test]
fn render_jinja_markdown_file_supports_nested_json_context() {
    let temp = fixture_bundle();
    let template_engine = configured_template_engine(temp.path());
    let renderer = configured_renderer(temp.path());
    let html = template_engine
        .render(template_request(
            TemplateInput::File("article.jinja.md".to_string()),
            TemplateContentKind::JinjaMarkdown,
            Some("base16-ocean.dark".to_string()),
            Some(sample_context_json()),
        ))
        .expect("render jinja markdown");

    let rendered = renderer
        .render(html_request(html, ImageFormat::Png))
        .expect("render jinja markdown html");

    assert!(!rendered.bytes.is_empty());
    let decoded = image::load_from_memory_with_format(&rendered.bytes, DecodedImageFormat::Png)
        .expect("decode png bytes");
    assert_eq!(decoded.width(), 64);
    assert_eq!(decoded.height(), 64);
}

#[test]
fn renderer_search_paths_resolve_relative_assets_from_root_only() {
    let temp = fixture_bundle();
    fs::create_dir_all(temp.path().join("pages")).expect("create pages dir");
    fs::write(
        temp.path().join("pages/root-relative.jinja"),
        r#"<!doctype html>
<html>
  <body>
    <img src="pages/nested-only.png" width="1" height="1" />
  </body>
</html>
"#,
    )
    .expect("write root-relative template");
    fs::write(
        temp.path().join("pages/template-relative.jinja"),
        r#"<!doctype html>
<html>
  <body>
    <img src="nested-only.png" width="1" height="1" />
  </body>
</html>
"#,
    )
    .expect("write template-relative template");
    fs::copy(
        temp.path().join("pixel.png"),
        temp.path().join("pages/nested-only.png"),
    )
    .expect("copy nested pixel");

    let template_engine = configured_template_engine(temp.path());
    let renderer = configured_renderer(temp.path());

    let root_relative_html = template_engine
        .render(template_request(
            TemplateInput::File("pages/root-relative.jinja".to_string()),
            TemplateContentKind::JinjaHtml,
            None,
            Some(sample_context_json()),
        ))
        .expect("render root-relative template");
    let rendered = renderer
        .render(html_request(root_relative_html, ImageFormat::Png))
        .expect("render root-relative html");
    assert!(!rendered.bytes.is_empty());

    let template_relative_html = template_engine
        .render(template_request(
            TemplateInput::File("pages/template-relative.jinja".to_string()),
            TemplateContentKind::JinjaHtml,
            None,
            Some(sample_context_json()),
        ))
        .expect("render template-relative template");
    let error = renderer
        .render(html_request(template_relative_html, ImageFormat::Png))
        .expect_err("template-directory-relative asset should fail");
    assert!(error.to_string().contains("failed to read asset"));
}

#[test]
fn render_without_fixed_height_uses_measured_content_height() {
    let temp = fixture_bundle();
    let template_engine = configured_template_engine(temp.path());
    let renderer = configured_renderer(temp.path());
    let html = template_engine
        .render(template_request(
            TemplateInput::File("index.jinja".to_string()),
            TemplateContentKind::JinjaHtml,
            None,
            Some(sample_context_json()),
        ))
        .expect("render template");
    let mut request = html_request(html, ImageFormat::Png);
    request.viewport.height = None;

    let measured = renderer
        .measure(request.clone())
        .expect("measure auto-height layout");
    let rendered = renderer.render(request).expect("render auto-height layout");

    assert_eq!(measured.width, 64);
    assert_eq!(measured.height, 64);
    assert_eq!(rendered.width, 64);
    assert_eq!(rendered.height, 64);
}

fn configured_template_engine(search_path: &Path) -> std::sync::Arc<TemplateEngine> {
    let engine = TemplateEngine::new();
    engine
        .add_search_path(search_path.to_string_lossy().into_owned())
        .expect("add search path");
    engine
}

fn configured_renderer(search_path: &Path) -> std::sync::Arc<Renderer> {
    let renderer = Renderer::new();
    renderer
        .add_search_path(search_path.to_string_lossy().into_owned())
        .expect("add search path");
    renderer
        .add_font_file(font_path().to_string_lossy().into_owned())
        .expect("load fixture font");
    renderer
}

fn template_request(
    input: TemplateInput,
    content_kind: TemplateContentKind,
    syntax_theme: Option<String>,
    context_json: Option<String>,
) -> TemplateRequest {
    TemplateRequest {
        input,
        context_json,
        content_kind,
        syntax_theme,
    }
}

fn html_request(html: String, format: ImageFormat) -> RenderRequest {
    RenderRequest {
        input: RenderInput::Inline(html),
        viewport: RenderSize {
            width: Some(64),
            height: Some(64),
            device_pixel_ratio: None,
        },
        format,
        quality: Some(100),
        load_linked_stylesheets: None,
        normalize_whitespace: None,
    }
}

fn inline_render_request(
    markup: &str,
    width: u32,
    height: u32,
    format: ImageFormat,
    quality: Option<u8>,
) -> RenderRequest {
    RenderRequest {
        input: RenderInput::Inline(markup.to_string()),
        viewport: RenderSize {
            width: Some(width),
            height: Some(height),
            device_pixel_ratio: None,
        },
        format,
        quality,
        load_linked_stylesheets: None,
        normalize_whitespace: None,
    }
}

fn render_inline_png(markup: &str, width: u32, height: u32) -> image::RgbaImage {
    let renderer = Renderer::new();
    renderer
        .add_font_file(font_path().to_string_lossy().into_owned())
        .expect("load fixture font");
    let rendered = renderer
        .render(inline_render_request(
            markup,
            width,
            height,
            ImageFormat::Png,
            None,
        ))
        .expect("render CSS fixture");
    image::load_from_memory_with_format(&rendered.bytes, DecodedImageFormat::Png)
        .expect("decode CSS fixture")
        .to_rgba8()
}

fn contains_webp_chunk(bytes: &[u8], chunk: &[u8; 4]) -> bool {
    bytes.windows(chunk.len()).any(|window| window == chunk)
}

fn count_non_white_pixels(image: &image::RgbaImage, start_y: u32, end_y: u32) -> usize {
    (start_y..end_y)
        .flat_map(|y| (0..image.width()).map(move |x| image.get_pixel(x, y).0))
        .filter(|pixel| *pixel != [255, 255, 255, 255])
        .count()
}

fn sample_context_json() -> String {
    json!({
        "user": {
            "profile": {
                "display_name": "Takumi"
            }
        }
    })
    .to_string()
}

fn fixture_bundle() -> TempDir {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join("pixel.png"), tiny_png_bytes()).expect("write pixel png");
    fs::write(
        temp.path().join("styles.css"),
        r#"
html, body {
  width: 64px;
  height: 64px;
  margin: 0;
}

body {
  background: #111827;
  background-image: url('./pixel.png');
  background-repeat: repeat;
}

.card {
  width: 64px;
  height: 64px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #f9fafb;
  font-family: "Rubik", sans-serif;
  font-size: 12px;
}
"#,
    )
    .expect("write styles");
    fs::write(
        temp.path().join("index.jinja"),
        r#"<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <link rel="stylesheet" href="styles.css" />
  </head>
  <body>
    <div class="card">{{ user.profile.display_name }}</div>
    <img src="pixel.png" width="1" height="1" />
  </body>
</html>
"#,
    )
    .expect("write template");
    fs::write(
        temp.path().join("post.md"),
        r#"<link rel=\"stylesheet\" href=\"styles.css\" />

# Plain Markdown example

This screenshot is rendered from **Markdown** without any `context_json`.

![Pixel](pixel.png)

```rust
let request = RenderHtmlRequest {
    input: HtmlInput::Html(html),
    viewport: RenderSize {
        width: Some(64),
        height: Some(64),
        device_pixel_ratio: None,
    },
    format: ImageFormat::Png,
    quality: Some(100),
    load_linked_stylesheets: None,
    normalize_whitespace: None,
};
```
"#,
    )
    .expect("write markdown post");
    fs::write(
        temp.path().join("static-article.jinja.md"),
        r#"<link rel=\"stylesheet\" href=\"styles.css\" />

# Static JinjaMarkdown example

No template variables are required here, so `context_json` can stay omitted.

![Pixel](pixel.png)
"#,
    )
    .expect("write static jinja markdown article");
    fs::write(
        temp.path().join("article.jinja.md"),
        r#"<link rel=\"stylesheet\" href=\"styles.css\" />

# {{ user.profile.display_name }}

This article is rendered from **JinjaMarkdown** with nested JSON context.

![Pixel](pixel.png)
"#,
    )
    .expect("write jinja markdown article");
    temp
}

fn font_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/fonts/Rubik-Regular.ttf")
}

fn tiny_png_bytes() -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 255, 255, 255]));
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgba8(image)
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .expect("encode tiny png");
    bytes
}
