#![recursion_limit = "512"]

use std::{error::Error, path::PathBuf};

use serde_json::json;
use takumi_render_uniffi::{
    RenderInput, ImageFormat, RenderRequest, RenderSize, TemplateRequest, Renderer,
    TemplateContentKind, TemplateEngine, TemplateInput,
};

fn main() -> Result<(), Box<dyn Error>> {
    let template_engine = TemplateEngine::new();
    let renderer = Renderer::new();
    let asset_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/assets").canonicalize().unwrap();
    let asset_dir = asset_root.join("jinjaMarkdownArticle");
    let output_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("output-jinja-markdown-article.png");

    template_engine.add_search_path(asset_dir.to_string_lossy().into_owned())?;
    renderer.add_search_path(asset_dir.to_string_lossy().into_owned())?;
    renderer.add_font_directory(asset_root.join("fonts").to_string_lossy().into_owned())?;

    let html = template_engine.render(TemplateRequest {
        input: TemplateInput::File("index.jinja.md".to_string()),
        context_json: Some(sample_context().to_string()),
        content_kind: TemplateContentKind::JinjaMarkdown,
        syntax_theme: Some("base16-ocean.dark".to_string()),
    })?;

    renderer.render_to_file(
        RenderRequest {
            input: RenderInput::Inline(html),
            viewport: RenderSize {
                width: Some(1200),
                height: Some(800),
                device_pixel_ratio: None,
            },
            format: ImageFormat::Png,
            quality: None,
            load_linked_stylesheets: None,
            normalize_whitespace: None,
        },
        output_path.to_string_lossy().into_owned(),
    )?;

    println!("rendered {}", output_path.display());
    Ok(())
}

fn sample_context() -> serde_json::Value {
    json!({
        "cover_src": "../110305269_p0.jpg",
        "title": "JinjaMarkdown keeps templating and prose in one place",
        "subtitle": "Render Markdown after Jinja without forcing plain Markdown callers to provide context.",
        "author_name": "Takumi examples",
        "published_at": "2026-05-25",
        "intro": "This sample shows how nested JSON can populate headings, summaries, and snippets inside a Markdown document.",
        "highlights": [
            "`context_json` is `Some(json)` only when the template needs variables.",
            "Relative assets must now be written relative to the renderer search path root."
        ],
        "code_sample": "println!(\"Hello, world!\"); // This is a Rust code snippet inside the Markdown content.",
    })
}
