use std::{fs, path::{Path, PathBuf}};

use image::ImageFormat as DecodedImageFormat;
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
    let template_source = fs::read_to_string(temp.path().join("index.jinja")).expect("read fixture template");
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

    let rendered = renderer
        .render(request)
        .expect("render template string");

    assert!(!rendered.bytes.is_empty());
    assert_eq!(rendered.format, ImageFormat::Png);
    let decoded = image::load_from_memory_with_format(&rendered.bytes, DecodedImageFormat::Png)
        .expect("decode png bytes");
    assert_eq!(decoded.width(), 64);
    assert_eq!(decoded.height(), 64);
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
            value: temp.path().join("index.jinja").to_string_lossy().into_owned(),
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
    let template_source = fs::read_to_string(temp.path().join("index.jinja")).expect("read fixture template");
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
    RenderRequest {
        input,
        context_json: json!({
            "user": {
                "profile": {
                    "display_name": "Takumi"
                }
            }
        })
        .to_string(),
        viewport: RenderSize {
            width: 64,
            height: 64,
        },
        format,
        quality: Some(100),
        load_linked_stylesheets: None,
        resolve_local_assets: None,
        normalize_whitespace: None,
    }
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
    temp
}

fn font_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/fonts/Rubik-Regular.ttf")
}

fn tiny_png_bytes() -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 255, 255, 255]));
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)
        .expect("encode tiny png");
    bytes
}
