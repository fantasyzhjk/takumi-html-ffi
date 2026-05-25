use std::{
    borrow::Cow,
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
};

use takumi::{
    GlobalContext,
    layout::Viewport,
    rendering::{RenderOptions, render as takumi_render, write_image},
    resources::{font::FontResource, image::ImageSource},
};
use takumi_html::{
    FromHtmlOptions, FromHtmlResult, HtmlError, LocalAssetMode, from_document_with_options,
};

use crate::{
    api::{RenderRequest, RenderedImage},
    cache::{FileCache, FontCache, absolute_path, hash_bytes, normalize_existing_path},
    error::{RendererError, Result},
    template::{TemplateRepository, render_markup},
};

#[derive(uniffi::Object)]
pub struct Renderer {
    global: Mutex<GlobalContext>,
    search_paths: RwLock<Vec<PathBuf>>,
    registered_templates: RwLock<HashMap<String, String>>,
    file_cache: Arc<Mutex<FileCache>>,
    font_cache: Mutex<FontCache>,
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            global: Mutex::new(GlobalContext::default()),
            search_paths: RwLock::new(Vec::new()),
            registered_templates: RwLock::new(HashMap::new()),
            file_cache: Arc::new(Mutex::new(FileCache::default())),
            font_cache: Mutex::new(FontCache::default()),
        }
    }
}

#[uniffi::export]
impl Renderer {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn add_search_path(&self, path: String) -> Result<()> {
        let normalized = normalize_search_path(&path)?;
        let mut search_paths = self
            .search_paths
            .write()
            .map_err(|_| state_lock_error("search_paths"))?;
        if !search_paths.iter().any(|existing| existing == &normalized) {
            search_paths.push(normalized);
        }
        Ok(())
    }

    pub fn add_template(&self, name: String, source: String) -> Result<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err(RendererError::invalid_request("template name cannot be empty"));
        }

        self.registered_templates
            .write()
            .map_err(|_| state_lock_error("registered_templates"))?
            .insert(name.to_string(), source);
        Ok(())
    }

    pub fn clear_templates(&self) -> Result<()> {
        self.registered_templates
            .write()
            .map_err(|_| state_lock_error("registered_templates"))?
            .clear();
        Ok(())
    }

    pub fn add_font_file(&self, path: String) -> Result<()> {
        self.add_font_file_impl(Path::new(path.trim()))
    }

    pub fn add_font_bytes(&self, bytes: Vec<u8>) -> Result<()> {
        self.add_font_bytes_impl(bytes)
    }

    pub fn clear_caches(&self) -> Result<()> {
        *self.global.lock().map_err(|_| state_lock_error("global"))? = GlobalContext::default();
        self.file_cache
            .lock()
            .map_err(|_| state_lock_error("file_cache"))?
            .clear();
        self.font_cache
            .lock()
            .map_err(|_| state_lock_error("font_cache"))?
            .clear();
        Ok(())
    }

    pub fn render(&self, request: RenderRequest) -> Result<RenderedImage> {
        self.render_request(request)
    }

    pub fn render_to_file(&self, request: RenderRequest, output_path: String) -> Result<RenderedImage> {
        let rendered = self.render_request(request)?;
        self.write_rendered_image_to_path(&rendered, &output_path)?;
        Ok(rendered)
    }

    pub fn render_template_string(
        &self,
        template_source: String,
        mut request: RenderRequest,
    ) -> Result<RenderedImage> {
        request.template_source = Some(template_source);
        request.template_file = None;
        self.render_request(request)
    }

    pub fn render_template_string_to_file(
        &self,
        template_source: String,
        mut request: RenderRequest,
        output_path: String,
    ) -> Result<RenderedImage> {
        request.template_source = Some(template_source);
        request.template_file = None;
        let rendered = self.render_request(request)?;
        self.write_rendered_image_to_path(&rendered, &output_path)?;
        Ok(rendered)
    }

    pub fn render_template_file(
        &self,
        template_path: String,
        mut request: RenderRequest,
    ) -> Result<RenderedImage> {
        request.template_source = None;
        request.template_name = None;
        request.template_file = Some(template_path);
        self.render_request(request)
    }

    pub fn render_template_file_to_file(
        &self,
        template_path: String,
        mut request: RenderRequest,
        output_path: String,
    ) -> Result<RenderedImage> {
        request.template_source = None;
        request.template_name = None;
        request.template_file = Some(template_path);
        let rendered = self.render_request(request)?;
        self.write_rendered_image_to_path(&rendered, &output_path)?;
        Ok(rendered)
    }

    pub fn render_template_name(
        &self,
        template_name: String,
        mut request: RenderRequest,
    ) -> Result<RenderedImage> {
        request.template_source = None;
        request.template_file = None;
        request.template_name = Some(template_name);
        self.render_request(request)
    }

    pub fn render_template_name_to_file(
        &self,
        template_name: String,
        mut request: RenderRequest,
        output_path: String,
    ) -> Result<RenderedImage> {
        request.template_source = None;
        request.template_file = None;
        request.template_name = Some(template_name);
        let rendered = self.render_request(request)?;
        self.write_rendered_image_to_path(&rendered, &output_path)?;
        Ok(rendered)
    }
}

impl Renderer {
    #[doc(hidden)]
    pub fn debug_font_cache_entries(&self) -> usize {
        self.font_cache
            .lock()
            .map(|cache| cache.entry_count())
            .unwrap_or_default()
    }

    fn render_request(&self, request: RenderRequest) -> Result<RenderedImage> {
        validate_request(&request)?;

        let repository = self.template_repository()?;
        let rendered_markup = render_markup(&request, repository)?;
        let html_result = self.convert_markup(
            &rendered_markup.markup,
            &request,
            &rendered_markup.base_candidates,
        )?;
        let fetched_resources = self.preload_fetched_resources(&html_result)?;
        let stylesheet = html_result.stylesheet();
        let node = html_result.node;
        let viewport = Viewport::new((request.viewport.width, request.viewport.height));
        let global = self.global.lock().map_err(|_| state_lock_error("global"))?;
        let image = takumi_render(
            RenderOptions::builder()
                .viewport(viewport)
                .node(node)
                .fetched_resources(fetched_resources)
                .stylesheet(stylesheet)
                .global(&global)
                .build(),
        )?;
        let bytes = self.encode_image(&image, &request)?;

        Ok(RenderedImage {
            bytes,
            format: request.format,
            width: image.width(),
            height: image.height(),
            content_type: Some(request.format.content_type().to_string()),
        })
    }

    fn convert_markup(
        &self,
        markup: &str,
        request: &RenderRequest,
        base_candidates: &[PathBuf],
    ) -> Result<FromHtmlResult> {
        if base_candidates.is_empty() {
            return self
                .convert_markup_with_base(markup, request, None)
                .map_err(Into::into);
        }

        let mut first_retryable_error: Option<HtmlError> = None;
        for base_path in dedup_paths(base_candidates) {
            match self.convert_markup_with_base(markup, request, Some(base_path.as_path())) {
                Ok(result) => return Ok(result),
                Err(error) if can_retry_with_next_base(&error) => {
                    if first_retryable_error.is_none() {
                        first_retryable_error = Some(error);
                    }
                }
                Err(error) => return Err(error.into()),
            }
        }

        match first_retryable_error {
            Some(error) => Err(error.into()),
            None => Err(RendererError::invalid_request(
                "no usable base path candidates were available for relative assets",
            )),
        }
    }

    fn convert_markup_with_base(
        &self,
        markup: &str,
        request: &RenderRequest,
        base_path: Option<&Path>,
    ) -> std::result::Result<FromHtmlResult, HtmlError> {
        let local_asset_mode = requested_local_asset_mode(request);
        let mut options = FromHtmlOptions::new()
            .load_linked_stylesheets(request.load_linked_stylesheets.unwrap_or(true))
            .normalize_whitespace(request.normalize_whitespace.unwrap_or(true));

        options = match local_asset_mode {
            RequestedLocalAssetMode::PreserveRaw => options.resolve_local_assets(false),
            RequestedLocalAssetMode::InlineDataUri => options.resolve_local_assets(true),
            RequestedLocalAssetMode::CacheByAbsolutePath => {
                options.local_asset_mode(LocalAssetMode::AbsolutePath)
            }
        };

        if let Some(base_path) = base_path {
            options = options.with_base_path(base_path);
        }

        from_document_with_options(markup, &options)
    }

    fn preload_fetched_resources(&self, html_result: &FromHtmlResult) -> Result<HashMap<Arc<str>, ImageSource>> {
        let mut fetched_resources = HashMap::new();
        let rendered_node_html = html_result.node.to_html();

        for url in collect_html_resource_urls(&rendered_node_html) {
            self.maybe_insert_fetched_resource(&mut fetched_resources, &url)?;
        }

        for url in collect_stylesheet_resource_urls(html_result.stylesheet_sources()) {
            self.maybe_insert_fetched_resource(&mut fetched_resources, &url)?;
        }

        Ok(fetched_resources)
    }

    fn maybe_insert_fetched_resource(
        &self,
        fetched_resources: &mut HashMap<Arc<str>, ImageSource>,
        raw_url: &str,
    ) -> Result<()> {
        if fetched_resources.contains_key(raw_url) || !is_local_cached_asset_reference(raw_url) {
            return Ok(());
        }

        let image = self.load_cached_image(raw_url)?;
        fetched_resources.insert(Arc::<str>::from(raw_url), image);
        Ok(())
    }

    fn load_cached_image(&self, raw_url: &str) -> Result<ImageSource> {
        let (path_part, _) = split_reference_suffix(raw_url);
        let normalized = normalize_existing_path(&cached_asset_reference_path(path_part))?;
        let cache_key = normalized.to_string_lossy().replace('\\', "/");

        if let Some(image) = self
            .global
            .lock()
            .map_err(|_| state_lock_error("global"))?
            .persistent_image_store
            .get(&cache_key)
        {
            return Ok(image);
        }

        let bytes = {
            let mut file_cache = self
                .file_cache
                .lock()
                .map_err(|_| state_lock_error("file_cache"))?;
            file_cache.read_bytes(&normalized)?
        };
        let image = ImageSource::from_bytes(bytes.as_ref().as_slice())
            .map_err(|error| RendererError::Render(error.to_string()))?;

        self.global
            .lock()
            .map_err(|_| state_lock_error("global"))?
            .persistent_image_store
            .insert(cache_key, image.clone());

        Ok(image)
    }

    fn encode_image(&self, image: &image::RgbaImage, request: &RenderRequest) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        write_image(
            Cow::Borrowed(image),
            &mut bytes,
            request.format.into_output_format(),
            request.quality.map(|value| value.min(100)),
        )
        .map_err(|error| RendererError::encode(error.to_string()))?;
        Ok(bytes)
    }

    fn template_repository(&self) -> Result<TemplateRepository> {
        let search_paths = self
            .search_paths
            .read()
            .map_err(|_| state_lock_error("search_paths"))?
            .clone();
        let registered_templates = self
            .registered_templates
            .read()
            .map_err(|_| state_lock_error("registered_templates"))?
            .clone();

        Ok(TemplateRepository {
            search_paths,
            registered_templates,
            file_cache: Arc::clone(&self.file_cache),
        })
    }

    fn add_font_file_impl(&self, path: &Path) -> Result<()> {
        let normalized = normalize_existing_path(path)?;
        if !normalized.is_file() {
            return Err(RendererError::invalid_request(format!(
                "font path `{}` is not a file",
                normalized.display()
            )));
        }

        {
            let font_cache = self
                .font_cache
                .lock()
                .map_err(|_| state_lock_error("font_cache"))?;
            if font_cache.contains_path(&normalized) {
                return Ok(());
            }
        }

        let bytes = {
            let mut file_cache = self
                .file_cache
                .lock()
                .map_err(|_| state_lock_error("file_cache"))?;
            file_cache.read_bytes(&normalized)?
        };
        let hash = hash_bytes(bytes.as_ref());

        {
            let mut font_cache = self
                .font_cache
                .lock()
                .map_err(|_| state_lock_error("font_cache"))?;
            if font_cache.contains_path(&normalized) {
                return Ok(());
            }
            if font_cache.contains_hash(hash) {
                font_cache.remember_path(normalized);
                return Ok(());
            }
        }

        self.global
            .lock()
            .map_err(|_| state_lock_error("global"))?
            .font_context
            .load_and_store(FontResource::new(bytes.as_ref().as_slice()))?;

        let mut font_cache = self
            .font_cache
            .lock()
            .map_err(|_| state_lock_error("font_cache"))?;
        font_cache.remember_hash(hash);
        font_cache.remember_path(normalized);
        Ok(())
    }

    fn add_font_bytes_impl(&self, bytes: Vec<u8>) -> Result<()> {
        let hash = hash_bytes(&bytes);
        {
            let font_cache = self
                .font_cache
                .lock()
                .map_err(|_| state_lock_error("font_cache"))?;
            if font_cache.contains_hash(hash) {
                return Ok(());
            }
        }

        self.global
            .lock()
            .map_err(|_| state_lock_error("global"))?
            .font_context
            .load_and_store(FontResource::new(bytes.as_slice()))?;

        self.font_cache
            .lock()
            .map_err(|_| state_lock_error("font_cache"))?
            .remember_hash(hash);
        Ok(())
    }

    fn write_rendered_image_to_path(&self, rendered: &RenderedImage, output_path: &str) -> Result<()> {
        let output_path = absolute_path(Path::new(output_path.trim()))?;
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(output_path, &rendered.bytes)?;
        Ok(())
    }
}

fn validate_request(request: &RenderRequest) -> Result<()> {
    if request.viewport.width == 0 || request.viewport.height == 0 {
        return Err(RendererError::invalid_request(
            "viewport width and height must both be greater than zero",
        ));
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequestedLocalAssetMode {
    PreserveRaw,
    InlineDataUri,
    CacheByAbsolutePath,
}

fn requested_local_asset_mode(request: &RenderRequest) -> RequestedLocalAssetMode {
    match request.resolve_local_assets {
        Some(true) => RequestedLocalAssetMode::InlineDataUri,
        Some(false) => RequestedLocalAssetMode::PreserveRaw,
        None => RequestedLocalAssetMode::CacheByAbsolutePath,
    }
}

fn collect_stylesheet_resource_urls(stylesheets: &[String]) -> Vec<String> {
    let mut urls = Vec::new();

    for stylesheet in stylesheets {
        let mut cursor = 0;
        while cursor < stylesheet.len() {
            if let Some((token, end_index)) = parse_url_token(stylesheet, cursor) {
                urls.push(token);
                cursor = end_index;
                continue;
            }

            let ch = stylesheet[cursor..]
                .chars()
                .next()
                .expect("cursor always points to a valid character boundary");
            cursor += ch.len_utf8();
        }
    }

    urls
}

fn collect_html_resource_urls(markup: &str) -> Vec<String> {
    let mut urls = collect_stylesheet_resource_urls(&[markup.to_string()]);
    let mut cursor = 0;

    while let Some(relative_start) = markup[cursor..].find("src=\"") {
        let value_start = cursor + relative_start + 5;
        let Some(relative_end) = markup[value_start..].find('"') else {
            break;
        };
        let value_end = value_start + relative_end;
        urls.push(markup[value_start..value_end].to_string());
        cursor = value_end + 1;
    }

    urls
}

fn parse_url_token(source: &str, start: usize) -> Option<(String, usize)> {
    let bytes = source.as_bytes();
    if bytes.get(start..start + 3)?.eq_ignore_ascii_case(b"url") {
        let mut cursor = start + 3;
        while matches!(bytes.get(cursor), Some(b' ' | b'\n' | b'\r' | b'\t' | 0x0C)) {
            cursor += 1;
        }

        if !matches!(bytes.get(cursor), Some(b'(')) {
            return None;
        }
        cursor += 1;

        while matches!(bytes.get(cursor), Some(b' ' | b'\n' | b'\r' | b'\t' | 0x0C)) {
            cursor += 1;
        }

        let quote = match bytes.get(cursor) {
            Some(b'\'') => Some('\''),
            Some(b'\"') => Some('\"'),
            _ => None,
        };

        if quote.is_some() {
            cursor += 1;
        }

        let value_start = cursor;
        while let Some(ch) = source[cursor..].chars().next() {
            if let Some(quote) = quote {
                if ch == quote {
                    let value = source[value_start..cursor].to_owned();
                    cursor += ch.len_utf8();
                    while matches!(bytes.get(cursor), Some(b' ' | b'\n' | b'\r' | b'\t' | 0x0C)) {
                        cursor += 1;
                    }
                    if matches!(bytes.get(cursor), Some(b')')) {
                        return Some((value, cursor + 1));
                    }
                    return None;
                }
            } else if ch == ')' {
                let value = source[value_start..cursor].trim().to_owned();
                return Some((value, cursor + 1));
            }
            cursor += ch.len_utf8();
        }
    }

    None
}

fn split_reference_suffix(reference: &str) -> (&str, &str) {
    let search_start = if starts_with_windows_verbatim_prefix(reference) {
        4
    } else {
        0
    };
    let split_at = reference[search_start..]
        .find(['?', '#'])
        .map(|index| index + search_start)
        .unwrap_or(reference.len());
    reference.split_at(split_at)
}

fn starts_with_windows_verbatim_prefix(reference: &str) -> bool {
    reference.starts_with("//?/")
        || reference.starts_with("//./")
        || reference.starts_with(r"\\?\")
        || reference.starts_with(r"\\.\")
}

fn cached_asset_reference_path(path_part: &str) -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(rest) = path_part.strip_prefix("//?/") {
            return PathBuf::from(format!(r"\\?\{}", rest.replace('/', "\\")));
        }

        if let Some(rest) = path_part.strip_prefix("//./") {
            return PathBuf::from(format!(r"\\.\{}", rest.replace('/', "\\")));
        }
    }

    PathBuf::from(path_part)
}

#[cfg(windows)]
fn is_windows_verbatim_path(path_part: &str) -> bool {
    let normalized = path_part.replace('\\', "/");
    normalized.starts_with("//?/") || normalized.starts_with("//./")
}

#[cfg(not(windows))]
fn is_windows_verbatim_path(_: &str) -> bool {
    false
}

fn is_local_cached_asset_reference(reference: &str) -> bool {
    let (path_part, _) = split_reference_suffix(reference.trim());
    !path_part.is_empty()
        && (cached_asset_reference_path(path_part).is_absolute()
            || is_windows_verbatim_path(path_part))
}

fn normalize_search_path(path: &str) -> Result<PathBuf> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(RendererError::invalid_request("search path cannot be empty"));
    }

    let normalized = normalize_existing_path(Path::new(trimmed))?;
    if !normalized.is_dir() {
        return Err(RendererError::invalid_request(format!(
            "search path `{}` is not a directory",
            normalized.display()
        )));
    }
    Ok(normalized)
}

fn dedup_paths(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut deduped = Vec::new();
    for path in paths {
        if !deduped.iter().any(|existing| existing == path) {
            deduped.push(path.clone());
        }
    }
    deduped
}

fn can_retry_with_next_base(error: &HtmlError) -> bool {
    matches!(
        error,
        HtmlError::AssetReadFailed { .. } | HtmlError::MissingBasePathForRelativeReference { .. }
    )
}

fn state_lock_error(name: &str) -> RendererError {
    RendererError::Render(format!("renderer state lock `{name}` was poisoned"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::api::{ImageFormat, RenderRequest, RenderSize};
    use crate::cache::normalize_existing_path;
    use super::{Renderer, collect_stylesheet_resource_urls, is_local_cached_asset_reference};
    use tempfile::tempdir;

    #[test]
    fn add_font_file_is_deduplicated_by_cache() {
        let renderer = Renderer::default();
        let font_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/fonts/Rubik-Regular.ttf");

        renderer
            .add_font_file_impl(&font_path)
            .expect("load font the first time");
        renderer
            .add_font_file_impl(&font_path)
            .expect("load font the second time");

        assert_eq!(renderer.debug_font_cache_entries(), 1);
    }

    #[test]
    fn default_render_request_loads_local_images_via_fetched_resources() {
        let temp = tempdir().expect("tempdir");
        fs::write(temp.path().join("relative.png"), tiny_png_bytes()).expect("write image");

        let renderer = Renderer::default();
        renderer
            .add_search_path(temp.path().to_string_lossy().into_owned())
            .expect("add search path");

        let rendered = renderer
            .render_template_string(
                r#"<!doctype html><html><body><img src="relative.png" width="1" height="1"></body></html>"#
                    .to_string(),
                RenderRequest {
                    template_name: None,
                    template_file: None,
                    template_source: None,
                    context_json: "{}".to_string(),
                    viewport: RenderSize { width: 20, height: 20 },
                    format: ImageFormat::Png,
                    quality: None,
                    load_linked_stylesheets: None,
                    resolve_local_assets: None,
                    normalize_whitespace: None,
                },
            )
            .expect("render local image");

        let decoded = image::load_from_memory_with_format(&rendered.bytes, image::ImageFormat::Png)
            .expect("decode rendered png");
        let has_red_pixel = decoded
            .to_rgba8()
            .pixels()
            .any(|pixel| pixel.0 == [255, 0, 0, 255]);
        assert!(has_red_pixel, "expected at least one rendered red pixel from linked stylesheet background image");
    }

    #[test]
    fn default_render_request_preloads_local_images_referenced_from_linked_stylesheets() {
        let temp = tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("styles")).expect("create styles dir");
        fs::write(temp.path().join("relative.png"), tiny_png_bytes()).expect("write image");
        fs::write(
            temp.path().join("styles/linked.css"),
            "body { margin: 0; } .panel { width: 1px; height: 1px; background-image: url('../relative.png'); }",
        )
        .expect("write stylesheet");

        let renderer = Renderer::default();
        renderer
            .add_search_path(temp.path().to_string_lossy().into_owned())
            .expect("add search path");

        let request = RenderRequest {
            template_name: None,
            template_file: None,
            template_source: None,
            context_json: "{}".to_string(),
            viewport: RenderSize { width: 20, height: 20 },
            format: ImageFormat::Png,
            quality: None,
            load_linked_stylesheets: None,
            resolve_local_assets: None,
            normalize_whitespace: None,
        };

        let html_result = renderer
            .convert_markup_with_base(
                r#"<!doctype html><html><head><link rel="stylesheet" href="styles/linked.css"></head><body><div class="panel"></div></body></html>"#,
                &request,
                Some(temp.path()),
            )
            .expect("convert markup");

        let expected = normalize_existing_path(&temp.path().join("relative.png"))
            .expect("normalize image path")
            .to_string_lossy()
            .replace('\\', "/");
        let stylesheet_urls = collect_stylesheet_resource_urls(html_result.stylesheet_sources());
        assert_eq!(stylesheet_urls, vec![expected.clone()]);

        let preloaded = renderer
            .preload_fetched_resources(&html_result)
            .expect("preload resources");
        assert!(
            preloaded.contains_key(expected.as_str()),
            "expected linked stylesheet image to be preloaded into fetched resources"
        );
    }

    #[cfg(windows)]
    #[test]
    fn verbatim_windows_paths_are_treated_as_local_cached_assets() {
        assert!(is_local_cached_asset_reference(
            "//?/C:/Users/example/image.png"
        ));
    }

    fn tiny_png_bytes() -> Vec<u8> {
        let image = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]));
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)
            .expect("encode tiny png");
        bytes
    }
}
