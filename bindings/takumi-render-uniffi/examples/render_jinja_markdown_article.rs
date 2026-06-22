#![recursion_limit = "512"]

use std::{path::PathBuf, thread};

use serde_json::json;
use takumi_render_uniffi::{
    EmojiSet, RenderInput, ImageFormat, RenderRequest, RenderSize, TemplateRequest, Renderer,
    TemplateContentKind, TemplateEngine, TemplateInput,
};

fn main() {
    let builder = thread::Builder::new().stack_size(16 * 1024 * 1024);
    let handle = builder.spawn(run).expect("spawn thread");
    handle.join().expect("join thread");
}

fn run() {
    let result = try_run();
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn try_run() -> Result<(), String> {
    let template_engine = TemplateEngine::new();
    let renderer = Renderer::new();
    let examples_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let asset_root = examples_root.join("assets");
    let asset_dir = asset_root.join("jinjaMarkdownArticle");
    let output_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("output-jinja-markdown-article.png");

    template_engine
        .add_search_path(asset_dir.to_string_lossy().into_owned())
        .map_err(|e| e.to_string())?;
    renderer
        .add_search_path(asset_dir.to_string_lossy().into_owned())
        .map_err(|e| e.to_string())?;
    renderer
        .add_font_directory(examples_root.join("fonts").to_string_lossy().into_owned())
        .map_err(|e| e.to_string())?;

    let html = template_engine
        .render(TemplateRequest {
            input: TemplateInput::File("index.jinja.md".to_string()),
            context_json: Some(sample_context().to_string()),
            content_kind: TemplateContentKind::JinjaMarkdown,
            syntax_theme: Some("base16-ocean.dark".to_string()),
        })
        .map_err(|e| e.to_string())?;

    renderer
        .render_to_file(
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
                emoji_type: Some(EmojiSet::Twemoji),
            },
            output_path.to_string_lossy().into_owned(),
        )
        .map_err(|e| e.to_string())?;

    println!("rendered {}", output_path.display());
    Ok(())
}

fn sample_context() -> serde_json::Value {
    json!({
        "cover_src": "https://assets.ppy.sh/beatmaps/2570031/covers/fullsize.jpg",
        "title": "🎉 JinjaMarkdown keeps templating and prose in one place",
        "subtitle": "Render Markdown after Jinja without forcing plain Markdown callers to provide context. ✨",
        "author_name": "Takumi examples",
        "published_at": "2026-05-25",
        "intro": "This sample shows how nested JSON can populate headings, summaries, and snippets inside a Markdown document. 🚀 Remote images and emoji now work out of the box!",
        "highlights": [
            "`context_json` is `Some(json)` only when the template needs variables.",
            "Remote images load from CDN 🌐",
            "Emoji are rendered as Twemoji images 🎨"
        ],
        "code_sample": "println!(\"Hello, world!\"); // This is a Rust code snippet inside the Markdown content.",
    })
}
