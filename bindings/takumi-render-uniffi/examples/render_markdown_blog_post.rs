#![recursion_limit = "512"]

use std::{error::Error, path::PathBuf};

use takumi_render_uniffi::{
    ImageFormat, RenderContentKind, RenderInput, RenderRequest, RenderSize, RenderSourceKind,
    Renderer,
};

fn main() -> Result<(), Box<dyn Error>> {
    let renderer = Renderer::new();
    let asset_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/assets");
    let asset_dir = asset_root.join("markdownBlogPost");
    let output_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("output-markdown-blog-post.png");

    renderer.add_search_path(asset_dir.to_string_lossy().into_owned())?;
    for font_path in [
        asset_root.join("fonts/Rubik-Regular.ttf"),
        asset_root.join("fonts/Rubik-Bold.ttf"),
    ] {
        renderer.add_font_file(font_path.to_string_lossy().into_owned())?;
    }

    let request = RenderRequest {
        context_json: None,
        viewport: RenderSize {
            width: Some(1200),
            height: Some(800),
        },
        format: ImageFormat::Png,
        input: RenderInput {
            source_kind: RenderSourceKind::File,
            content_kind: RenderContentKind::Markdown,
            value: "index.md".to_string(),
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
