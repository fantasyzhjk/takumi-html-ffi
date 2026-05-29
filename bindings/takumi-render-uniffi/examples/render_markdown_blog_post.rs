#![recursion_limit = "512"]

use std::{error::Error, path::PathBuf};

use takumi_render_uniffi::{
    ImageFormat, RenderHtmlRequest, RenderSize, RenderTemplateRequest, Renderer,
    TemplateContentKind, TemplateEngine, TemplateInput,
};

fn main() -> Result<(), Box<dyn Error>> {
    let template_engine = TemplateEngine::new();
    let renderer = Renderer::new();
    let asset_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/assets");
    let asset_dir = asset_root.join("markdownBlogPost");
    let output_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("output-markdown-blog-post.png");

    template_engine.add_search_path(asset_dir.to_string_lossy().into_owned())?;
    renderer.add_search_path(asset_dir.to_string_lossy().into_owned())?;
    renderer.add_font_directory(asset_root.join("fonts").to_string_lossy().into_owned())?;

    let html = template_engine.render(RenderTemplateRequest {
        input: TemplateInput::File("post.md".to_string()),
        context_json: None,
        content_kind: TemplateContentKind::Markdown,
        syntax_theme: Some("base16-ocean.dark".to_string()),
    })?;

    renderer.render_to_file(
        RenderHtmlRequest {
            html,
            viewport: RenderSize {
                width: Some(1200),
                height: Some(800),
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
