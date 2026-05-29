use std::{
    fs,
    path::{Path, PathBuf},
};

use image::{GenericImageView, ImageFormat as DecodedImageFormat};
use serde_json::json;
use takumi_render_uniffi::{
    ImageFormat, RenderContentKind, RenderInput, RenderRequest, RenderSize, RenderSourceKind,
    Renderer,
};
use tempfile::TempDir;

#[test]
fn render_template_string_supports_nested_json_and_search_paths() {
    let temp = fixture_bundle();
    let renderer = configured_renderer(temp.path());
    let template_source =
        fs::read_to_string(temp.path().join("index.jinja")).expect("read fixture template");
    let request = request(
        RenderInput {
            source_kind: RenderSourceKind::Inline,
            content_kind: RenderContentKind::JinjaHtml,
            value: template_source,
            logical_name: Some("inline/index.jinja".to_string()),
            base_path: None,
            search_paths: None,
            syntax_theme: None,
        },
        ImageFormat::Png,
    );

    let rendered = renderer.render(request).expect("render template string");

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
        input: RenderInput {
            source_kind: RenderSourceKind::Inline,
            content_kind: RenderContentKind::Html,
            value: r#"<style>
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
            logical_name: None,
            base_path: None,
            search_paths: None,
            syntax_theme: None,
        },
        context_json: None,
        viewport: RenderSize {
            width: Some(64),
            height: Some(16),
        },
        format: ImageFormat::Png,
        quality: Some(100),
        load_linked_stylesheets: None,
        resolve_local_assets: None,
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
    let renderer = configured_renderer(temp.path());
    let output_path = temp.path().join("out/rendered.webp");
    let request = request(
        RenderInput {
            source_kind: RenderSourceKind::File,
            content_kind: RenderContentKind::JinjaHtml,
            value: temp
                .path()
                .join("index.jinja")
                .to_string_lossy()
                .into_owned(),
            logical_name: None,
            base_path: None,
            search_paths: None,
            syntax_theme: None,
        },
        ImageFormat::WebP,
    );

    let rendered = renderer
        .render_to_file(request, output_path.to_string_lossy().into_owned())
        .expect("render template file to file");

    let output_bytes = fs::read(&output_path).expect("read output file");
    assert_eq!(output_bytes, rendered.bytes);
    let decoded = image::load_from_memory_with_format(&output_bytes, DecodedImageFormat::WebP)
        .expect("decode webp bytes");
    assert_eq!(decoded.width(), 64);
    assert_eq!(decoded.height(), 64);
}

#[test]
fn registered_templates_render_by_name() {
    let temp = fixture_bundle();
    let renderer = configured_renderer(temp.path());
    let template_source =
        fs::read_to_string(temp.path().join("index.jinja")).expect("read fixture template");
    renderer
        .add_template("cards/profile.jinja".to_string(), template_source)
        .expect("register template");
    let request = request(
        RenderInput {
            source_kind: RenderSourceKind::Registered,
            content_kind: RenderContentKind::JinjaHtml,
            value: "cards/profile.jinja".to_string(),
            logical_name: None,
            base_path: None,
            search_paths: None,
            syntax_theme: None,
        },
        ImageFormat::Png,
    );

    let rendered = renderer
        .render(request)
        .expect("render template by registered name");

    assert!(!rendered.bytes.is_empty());
    assert_eq!(rendered.content_type.as_deref(), Some("image/png"));
}

#[test]
fn font_cache_deduplicates_repeated_font_loads() {
    let temp = fixture_bundle();
    let renderer = Renderer::new();
    let font_path = font_path();

    renderer
        .add_search_path(temp.path().to_string_lossy().into_owned())
        .expect("add search path");
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
    let renderer = configured_renderer(temp.path());
    let request = request_with_context(
        RenderInput {
            source_kind: RenderSourceKind::File,
            content_kind: RenderContentKind::Markdown,
            value: "post.md".to_string(),
            logical_name: None,
            base_path: None,
            search_paths: None,
            syntax_theme: Some("base16-ocean.dark".to_string()),
        },
        ImageFormat::Png,
        None,
    );

    let rendered = renderer
        .render(request)
        .expect("render markdown file without context");

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
    let renderer = configured_renderer(temp.path());
    let request = request_with_context(
        RenderInput {
            source_kind: RenderSourceKind::File,
            content_kind: RenderContentKind::JinjaMarkdown,
            value: "static-article.jinja.md".to_string(),
            logical_name: None,
            base_path: None,
            search_paths: None,
            syntax_theme: Some("base16-ocean.dark".to_string()),
        },
        ImageFormat::Png,
        None,
    );

    let rendered = renderer
        .render(request)
        .expect("render static jinja markdown without context");

    assert!(!rendered.bytes.is_empty());
    let decoded = image::load_from_memory_with_format(&rendered.bytes, DecodedImageFormat::Png)
        .expect("decode png bytes");
    assert_eq!(decoded.width(), 64);
    assert_eq!(decoded.height(), 64);
}

#[test]
fn render_jinja_markdown_file_supports_nested_json_context() {
    let temp = fixture_bundle();
    let renderer = configured_renderer(temp.path());
    let request = request(
        RenderInput {
            source_kind: RenderSourceKind::File,
            content_kind: RenderContentKind::JinjaMarkdown,
            value: "article.jinja.md".to_string(),
            logical_name: None,
            base_path: None,
            search_paths: None,
            syntax_theme: Some("base16-ocean.dark".to_string()),
        },
        ImageFormat::Png,
    );

    let rendered = renderer
        .render(request)
        .expect("render jinja markdown with nested json context");

    assert!(!rendered.bytes.is_empty());
    let decoded = image::load_from_memory_with_format(&rendered.bytes, DecodedImageFormat::Png)
        .expect("decode png bytes");
    assert_eq!(decoded.width(), 64);
    assert_eq!(decoded.height(), 64);
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

fn request(input: RenderInput, format: ImageFormat) -> RenderRequest {
    request_with_context(input, format, Some(sample_context_json()))
}

fn request_with_context(
    input: RenderInput,
    format: ImageFormat,
    context_json: Option<String>,
) -> RenderRequest {
    RenderRequest {
        input,
        context_json,
        viewport: RenderSize {
            width: Some(64),
            height: Some(64),
        },
        format,
        quality: Some(100),
        load_linked_stylesheets: None,
        resolve_local_assets: None,
        normalize_whitespace: None,
    }
}

#[test]
fn render_without_fixed_height_uses_measured_content_height() {
    let temp = fixture_bundle();
    let renderer = configured_renderer(temp.path());
    let mut request = request(
        RenderInput {
            source_kind: RenderSourceKind::File,
            content_kind: RenderContentKind::JinjaHtml,
            value: temp
                .path()
                .join("index.jinja")
                .to_string_lossy()
                .into_owned(),
            logical_name: None,
            base_path: None,
            search_paths: None,
            syntax_theme: None,
        },
        ImageFormat::Png,
    );
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
let request = RenderRequest {
    context_json: None,
    ..Default::default()
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
