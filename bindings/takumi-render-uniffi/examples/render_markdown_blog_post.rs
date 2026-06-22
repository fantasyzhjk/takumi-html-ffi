#![recursion_limit = "512"]

use std::path::PathBuf;

use takumi_render_uniffi::{
    EmojiSet, RenderInput, ImageFormat, RenderRequest, RenderSize, TemplateRequest, Renderer,
    TemplateContentKind, TemplateEngine, TemplateInput,
};

fn main() {
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
    let asset_dir = asset_root.join("markdownBlogPost");
    let output_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("output-markdown-blog-post.png");

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
            input: TemplateInput::File("index.md".to_string()),
            context_json: None,
            content_kind: TemplateContentKind::Markdown,
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
