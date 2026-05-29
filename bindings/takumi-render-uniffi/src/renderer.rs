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
    rendering::{RenderOptions, measure_layout, render as takumi_render, write_image},
    resources::{font::FontResource, image::ImageSource},
};
use takumi_html::{
    FromHtmlOptions, FromHtmlResult, HtmlError, LocalAssetMode, from_document_with_options,
};

use crate::{
    api::{RenderInput, MeasuredLayout, RenderRequest, RenderSize, RenderedImage},
    cache::{FileCache, FontCache, absolute_path, hash_bytes, normalize_existing_path},
    error::{RendererError, Result},
    template::normalize_search_path,
};

#[derive(uniffi::Object)]
pub struct Renderer {
    global: Mutex<GlobalContext>,
    search_paths: RwLock<Vec<PathBuf>>,
    file_cache: Arc<Mutex<FileCache>>,
    font_cache: Mutex<FontCache>,
}

struct PreparedHtmlInput {
    markup: String,
    search_paths: Vec<PathBuf>,
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            global: Mutex::new(GlobalContext::default()),
            search_paths: RwLock::new(Vec::new()),
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

    pub fn add_font_file(&self, path: String) -> Result<()> {
        self.add_font_file_impl(Path::new(path.trim()))
    }

    pub fn add_font_bytes(&self, bytes: Vec<u8>) -> Result<()> {
        self.add_font_bytes_impl(bytes)
    }

    pub fn add_font_directory(&self, path: String) -> Result<()> {
        for font_path in collect_font_files(Path::new(path.trim()))? {
            self.add_font_file_impl(&font_path)?;
        }
        Ok(())
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

    pub fn render_to_file(
        &self,
        request: RenderRequest,
        output_path: String,
    ) -> Result<RenderedImage> {
        let rendered = self.render_request(request)?;
        self.write_rendered_image_to_path(&rendered, &output_path)?;
        Ok(rendered)
    }

    pub fn measure(&self, request: RenderRequest) -> Result<MeasuredLayout> {
        self.measure_request(request)
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
        let html_result = self.prepare_html_result(&request)?;
        let fetched_resources = self.preload_fetched_resources(&html_result)?;
        let stylesheet = html_result.stylesheet();
        let node = html_result.node;
        let global = self.global.lock().map_err(|_| state_lock_error("global"))?;
        let viewport = if request_has_auto_viewport(&request) {
            let measured = measure_layout(
                RenderOptions::builder()
                    .viewport(viewport_from_request(&request))
                    .node(node.clone())
                    .fetched_resources(fetched_resources.clone())
                    .stylesheet(stylesheet.clone())
                    .global(&global)
                    .build(),
            )?;
            let (width, height) = resolved_layout_size(&measured, request.viewport.clone());
            resolved_viewport_from_request(&request, width, height)
        } else {
            viewport_from_request(&request)
        };
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

    fn measure_request(&self, request: RenderRequest) -> Result<MeasuredLayout> {
        let html_result = self.prepare_html_result(&request)?;
        let fetched_resources = self.preload_fetched_resources(&html_result)?;
        let stylesheet = html_result.stylesheet();
        let node = html_result.node;
        let viewport = viewport_from_request(&request);
        let global = self.global.lock().map_err(|_| state_lock_error("global"))?;
        let measured = measure_layout(
            RenderOptions::builder()
                .viewport(viewport)
                .node(node)
                .fetched_resources(fetched_resources)
                .stylesheet(stylesheet)
                .global(&global)
                .build(),
        )?;
        let (width, height) = resolved_layout_size(&measured, request.viewport);

        Ok(MeasuredLayout { width, height })
    }

    fn prepare_html_result(&self, request: &RenderRequest) -> Result<FromHtmlResult> {
        validate_request(request)?;
        let prepared = self.resolve_markup_input(request)?;
        self.convert_markup(&prepared.markup, request, &prepared.search_paths)
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
                "no usable search paths were available for relative assets",
            )),
        }
    }

    fn convert_markup_with_base(
        &self,
        markup: &str,
        request: &RenderRequest,
        base_path: Option<&Path>,
    ) -> std::result::Result<FromHtmlResult, HtmlError> {
        let mut options = FromHtmlOptions::new()
            .load_linked_stylesheets(request.load_linked_stylesheets.unwrap_or(true))
            .normalize_whitespace(request.normalize_whitespace.unwrap_or(true))
            .local_asset_mode(LocalAssetMode::AbsolutePath);

        if let Some(base_path) = base_path {
            options = options.with_base_path(base_path);
        }

        from_document_with_options(markup, &options)
    }

    fn resolve_markup_input(&self, request: &RenderRequest) -> Result<PreparedHtmlInput> {
        let mut search_paths = self
            .search_paths
            .read()
            .map_err(|_| state_lock_error("search_paths"))?
            .clone();

        match &request.input {
            RenderInput::Inline(markup) => Ok(PreparedHtmlInput {
                markup: markup.clone(),
                search_paths,
            }),
            RenderInput::File(file) => {
                let normalized = normalize_existing_path(Path::new(file.trim()))?;
                if !normalized.is_file() {
                    return Err(RendererError::invalid_request(format!(
                        "html file `{}` is not a file",
                        normalized.display()
                    )));
                }

                let markup = {
                    let mut file_cache = self
                        .file_cache
                        .lock()
                        .map_err(|_| state_lock_error("file_cache"))?;
                    file_cache.read_string(&normalized)?
                };

                if let Some(parent) = normalized.parent() {
                    let parent = parent.to_path_buf();
                    if !search_paths.iter().any(|existing| existing == &parent) {
                        search_paths.insert(0, parent);
                    }
                }

                Ok(PreparedHtmlInput {
                    markup,
                    search_paths,
                })
            }
        }
    }

    fn preload_fetched_resources(
        &self,
        html_result: &FromHtmlResult,
    ) -> Result<HashMap<Arc<str>, ImageSource>> {
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

    fn encode_image(
        &self,
        image: &image::RgbaImage,
        request: &RenderRequest,
    ) -> Result<Vec<u8>> {
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

    fn write_rendered_image_to_path(
        &self,
        rendered: &RenderedImage,
        output_path: &str,
    ) -> Result<()> {
        let output_path = absolute_path(Path::new(output_path.trim()))?;
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(output_path, &rendered.bytes)?;
        Ok(())
    }
}

fn validate_request(request: &RenderRequest) -> Result<()> {
    if matches!(request.viewport.width, Some(0)) || matches!(request.viewport.height, Some(0)) {
        return Err(RendererError::invalid_request(
            "viewport width and height must be greater than zero when provided",
        ));
    }

    if let Some(device_pixel_ratio) = request.viewport.device_pixel_ratio
        && (!device_pixel_ratio.is_finite() || device_pixel_ratio <= 0.0)
    {
        return Err(RendererError::invalid_request(
            "viewport device_pixel_ratio must be greater than zero",
        ));
    }

    match &request.input {
        RenderInput::Inline(_) => {}
        RenderInput::File(path) if path.trim().is_empty() => {
            return Err(RendererError::invalid_request("input.file cannot be empty"));
        }
        RenderInput::File(_) => {}
    }

    Ok(())
}

fn viewport_from_request(request: &RenderRequest) -> Viewport {
    let mut viewport = Viewport::new((request.viewport.width, request.viewport.height));
    if let Some(device_pixel_ratio) = request.viewport.device_pixel_ratio {
        viewport = viewport.with_device_pixel_ratio(device_pixel_ratio);
    }
    viewport
}

fn resolved_viewport_from_request(
    request: &RenderRequest,
    width: u32,
    height: u32,
) -> Viewport {
    let mut viewport = Viewport::new((width, height));
    if let Some(device_pixel_ratio) = request.viewport.device_pixel_ratio {
        viewport = viewport.with_device_pixel_ratio(device_pixel_ratio);
    }
    viewport
}

fn request_has_auto_viewport(request: &RenderRequest) -> bool {
    request.viewport.width.is_none() || request.viewport.height.is_none()
}

fn resolved_layout_size(
    measured: &takumi::rendering::MeasuredNode,
    viewport: RenderSize,
) -> (u32, u32) {
    let width = viewport
        .width
        .unwrap_or_else(|| measured.width.round().max(0.0) as u32);
    let height = viewport
        .height
        .unwrap_or_else(|| measured.height.round().max(0.0) as u32);
    (width, height)
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

fn collect_font_files(root: &Path) -> Result<Vec<PathBuf>> {
    let normalized = normalize_existing_path(root)?;
    if !normalized.is_dir() {
        return Err(RendererError::invalid_request(format!(
            "font directory `{}` is not a directory",
            normalized.display()
        )));
    }

    let mut font_files = Vec::new();
    collect_font_files_recursive(&normalized, &mut font_files)?;
    font_files.sort_by(|left, right| left.to_string_lossy().cmp(&right.to_string_lossy()));
    Ok(font_files)
}

fn collect_font_files_recursive(dir: &Path, font_files: &mut Vec<PathBuf>) -> Result<()> {
    let mut entries = fs::read_dir(dir)?.collect::<std::result::Result<Vec<_>, _>>()?;
    entries.sort_by(|left, right| {
        left.path()
            .to_string_lossy()
            .cmp(&right.path().to_string_lossy())
    });

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_font_files_recursive(&path, font_files)?;
            continue;
        }

        if is_supported_font_file(&path) {
            font_files.push(path);
        }
    }

    Ok(())
}

fn is_supported_font_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .is_some_and(|ext| matches!(ext.as_str(), "ttf" | "otf" | "ttc" | "otc"))
}

fn state_lock_error(name: &str) -> RendererError {
    RendererError::Render(format!("renderer state lock `{name}` was poisoned"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{Renderer, collect_font_files, collect_stylesheet_resource_urls};
    use crate::api::{RenderInput, ImageFormat, RenderRequest, RenderSize};
    use crate::cache::normalize_existing_path;
    use tempfile::tempdir;

    #[cfg(windows)]
    use super::is_local_cached_asset_reference;

    fn inline_html_request(markup: &str) -> RenderRequest {
        RenderRequest {
            input: RenderInput::Inline(markup.to_string()),
            viewport: RenderSize {
                width: Some(20),
                height: Some(20),
                device_pixel_ratio: None,
            },
            format: ImageFormat::Png,
            quality: None,
            load_linked_stylesheets: None,
            normalize_whitespace: None,
        }
    }

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
    fn add_font_directory_scans_recursively_and_deduplicates_by_hash() {
        let temp = tempdir().expect("tempdir");
        let nested = temp.path().join("nested");
        fs::create_dir_all(&nested).expect("create nested dir");
        let font_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/fonts/Rubik-Regular.ttf");
        fs::copy(&font_path, temp.path().join("a.ttf")).expect("copy font");
        fs::copy(&font_path, nested.join("b.ttf")).expect("copy font");
        fs::write(temp.path().join("ignore.txt"), b"noop").expect("write ignored file");

        let renderer = Renderer::default();
        renderer
            .add_font_directory(temp.path().to_string_lossy().into_owned())
            .expect("load font directory");

        assert_eq!(renderer.debug_font_cache_entries(), 1);
    }

    #[test]
    fn collect_font_files_ignores_non_font_extensions() {
        let temp = tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("fonts")).expect("create dir");
        fs::write(temp.path().join("fonts/ignore.txt"), b"noop").expect("write text");

        let collected = collect_font_files(temp.path()).expect("collect fonts");
        assert!(collected.is_empty());
    }

    #[test]
    fn renderer_search_paths_load_local_images_via_fetched_resources() {
        let temp = tempdir().expect("tempdir");
        fs::write(temp.path().join("relative.png"), tiny_png_bytes()).expect("write image");

        let renderer = Renderer::default();
        renderer
            .add_search_path(temp.path().to_string_lossy().into_owned())
            .expect("add search path");

        let rendered = renderer
            .render(inline_html_request(
                r#"<!doctype html><html><body><img src="relative.png" width="1" height="1"></body></html>"#,
            ))
            .expect("render local image");

        let decoded = image::load_from_memory_with_format(&rendered.bytes, image::ImageFormat::Png)
            .expect("decode rendered png");
        let has_red_pixel = decoded
            .to_rgba8()
            .pixels()
            .any(|pixel| pixel.0 == [255, 0, 0, 255]);
        assert!(
            has_red_pixel,
            "expected at least one rendered red pixel from linked stylesheet background image"
        );
    }

    #[test]
    fn renderer_retries_search_paths_for_relative_assets() {
        let temp = tempdir().expect("tempdir");
        let first_root = temp.path().join("first");
        let second_root = temp.path().join("second");
        fs::create_dir_all(&first_root).expect("create first root");
        fs::create_dir_all(&second_root).expect("create second root");
        fs::write(second_root.join("relative.png"), tiny_png_bytes()).expect("write image");

        let renderer = Renderer::default();
        renderer
            .add_search_path(first_root.to_string_lossy().into_owned())
            .expect("add first search path");
        renderer
            .add_search_path(second_root.to_string_lossy().into_owned())
            .expect("add second search path");

        let rendered = renderer
            .render(inline_html_request(
                r#"<!doctype html><html><body><img src="relative.png" width="1" height="1"></body></html>"#,
            ))
            .expect("render from fallback search path");

        assert!(!rendered.bytes.is_empty());
    }

    #[test]
    fn persistent_image_store_reuses_absolute_path_images() {
        let temp = tempdir().expect("tempdir");
        let image_path = temp.path().join("relative.png");
        fs::write(&image_path, tiny_png_bytes()).expect("write image");

        let renderer = Renderer::default();
        renderer
            .add_search_path(temp.path().to_string_lossy().into_owned())
            .expect("add search path");
        let request = inline_html_request(
            r#"<!doctype html><html><body><img src="relative.png" width="1" height="1"></body></html>"#,
        );

        renderer
            .render(request.clone())
            .expect("render local image the first time");
        let absolute_reference = normalize_existing_path(&image_path)
            .expect("normalize image path")
            .to_string_lossy()
            .replace('\\', "/");
        fs::remove_file(&image_path).expect("remove source image");

        renderer
            .load_cached_image(&absolute_reference)
            .expect("load image from persistent store");
    }

    #[test]
    fn request_file_uses_parent_directory_as_search_path() {
        let temp = tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("pages")).expect("create pages dir");
        fs::write(temp.path().join("pages/relative.png"), tiny_png_bytes()).expect("write image");
        fs::write(
            temp.path().join("pages/index.html"),
            r#"<!doctype html><html><body><img src="relative.png" width="1" height="1"></body></html>"#,
        )
        .expect("write html file");

        let renderer = Renderer::default();
        let rendered = renderer
            .render(RenderRequest {
                input: RenderInput::File(
                    temp.path()
                        .join("pages/index.html")
                        .to_string_lossy()
                        .into_owned(),
                ),
                viewport: RenderSize {
                    width: Some(20),
                    height: Some(20),
                    device_pixel_ratio: None,
                },
                format: ImageFormat::Png,
                quality: None,
                load_linked_stylesheets: None,
                normalize_whitespace: None,
            })
            .expect("render html file");

        let decoded = image::load_from_memory_with_format(&rendered.bytes, image::ImageFormat::Png)
            .expect("decode rendered png");
        let has_red_pixel = decoded
            .to_rgba8()
            .pixels()
            .any(|pixel| pixel.0 == [255, 0, 0, 255]);
        assert!(
            has_red_pixel,
            "expected relative asset from html file directory"
        );
    }

    #[test]
    fn viewport_device_pixel_ratio_scales_css_pixels() {
        let renderer = Renderer::default();
        let markup = r#"
<!doctype html>
<html>
  <head>
    <style>
      html, body { margin: 0; padding: 0; width: 32px; height: 16px; }
      .box { display: block; width: 16px; height: 16px; background: #ff0000; }
    </style>
  </head>
  <body>
    <div class="box"></div>
  </body>
</html>
"#;

        let rendered = renderer
            .render(RenderRequest {
                input: RenderInput::Inline(markup.to_string()),
                viewport: RenderSize {
                    width: Some(32),
                    height: Some(16),
                    device_pixel_ratio: Some(2.0),
                },
                format: ImageFormat::Png,
                quality: None,
                load_linked_stylesheets: None,
                normalize_whitespace: None,
            })
            .expect("render with dpr");

        let decoded = image::load_from_memory_with_format(&rendered.bytes, image::ImageFormat::Png)
            .expect("decode rendered png")
            .to_rgba8();
        assert_eq!(decoded.get_pixel(8, 8).0, [255, 0, 0, 255]);
        assert_eq!(decoded.get_pixel(24, 8).0, [255, 0, 0, 255]);
    }

    #[test]
    fn renderer_preloads_local_images_referenced_from_linked_stylesheets() {
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

        let request = inline_html_request(
            r#"<!doctype html><html><head><link rel="stylesheet" href="styles/linked.css"></head><body><div class="panel"></div></body></html>"#,
        );
        let markup = match &request.input {
            RenderInput::Inline(markup) => markup.as_str(),
            RenderInput::File(_) => unreachable!("inline_html_request always produces Html input"),
        };

        let html_result = renderer
            .convert_markup_with_base(markup, &request, Some(temp.path()))
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

    #[test]
    fn rejects_zero_viewport_dimensions() {
        let request = RenderRequest {
            input: RenderInput::Inline("<div />".to_string()),
            viewport: RenderSize {
                width: Some(0),
                height: Some(20),
                device_pixel_ratio: None,
            },
            format: ImageFormat::Png,
            quality: None,
            load_linked_stylesheets: None,
            normalize_whitespace: None,
        };

        let error = super::validate_request(&request).expect_err("reject zero width");
        assert!(error.to_string().contains("viewport width"));
    }

    fn tiny_png_bytes() -> Vec<u8> {
        let image = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]));
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgba8(image)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .expect("encode tiny png");
        bytes
    }
}
