#![recursion_limit = "512"]

use std::{error::Error, path::PathBuf};

use serde_json::json;
use takumi_render_uniffi::{
    ImageFormat, RenderContentKind, RenderInput, RenderRequest, RenderSize, RenderSourceKind,
    Renderer,
};

fn main() -> Result<(), Box<dyn Error>> {
    let renderer = Renderer::new();
    let asset_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/assets");
    let asset_dir = asset_root.join("jinjaMarkdownArticle");
    let output_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("output-jinja-markdown-article.png");

    renderer.add_search_path(asset_dir.to_string_lossy().into_owned())?;
    for font_path in [
        asset_root.join("fonts/Rubik-Regular.ttf"),
        asset_root.join("fonts/Rubik-Bold.ttf"),
    ] {
        renderer.add_font_file(font_path.to_string_lossy().into_owned())?;
    }

    let request = RenderRequest {
        context_json: Some(sample_context().to_string()),
        viewport: RenderSize {
            width: 1280,
            height: 720,
        },
        format: ImageFormat::Png,
        input: RenderInput {
            source_kind: RenderSourceKind::File,
            content_kind: RenderContentKind::JinjaMarkdown,
            value: "index.jinja.md".to_string(),
            logical_name: None,
            base_path: None,
            search_paths: None,
            syntax_theme: Some("base16-ocean.dark".to_string()),
        },
        quality: None,
        load_linked_stylesheets: None,
        resolve_local_assets: None,
        normalize_whitespace: None,
    };

    renderer.render_to_file(request, output_path.to_string_lossy().into_owned())?;

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
        "intro": "This sample shows how a nested JSON context can populate headings, summaries, and code snippets inside a Markdown document.",
        "highlights": [
            "`context_json` is `Some(json)` only when the template actually needs variables.",
            "Relative assets still resolve from the template directory.",
            "The Markdown code fence picks up the configured syntax theme."
        ],
        "code_sample": "let request = RenderRequest {\n    context_json: Some(sample_context().to_string()),\n    input: RenderInput {\n        content_kind: RenderContentKind::JinjaMarkdown,\n        ..input\n    },\n    ..request\n};"
    })
}
