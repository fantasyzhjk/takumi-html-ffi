use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
};

#[cfg(feature = "http")]
use reqwest::blocking::Client;
use takumi::prelude::{
    Bitmap, FontResource, Fonts, ImageSource, MeasuredNode, RenderOptions, Viewport,
};
use takumi::{measure, render as takumi_render, write_image};
use takumi_html::{
    FromHtmlOptions, FromHtmlResult, HtmlError, LocalAssetMode, from_document_with_options,
};

use crate::{
    api::{MeasuredLayout, RenderInput, RenderRequest, RenderSize, RenderedImage},
    cache::{
        FileCache, FileSignature, FontCache, ImageCache, absolute_path, hash_bytes,
        normalize_existing_path,
    },
    error::{RendererError, Result, state_lock_error},
    template::normalize_search_path,
};

#[derive(uniffi::Object)]
pub struct Renderer {
    fonts: Mutex<Fonts>,
    search_paths: RwLock<Vec<PathBuf>>,
    file_cache: Arc<Mutex<FileCache>>,
    font_cache: Mutex<FontCache>,
    image_cache: Mutex<ImageCache>,
    #[cfg(feature = "http")]
    http_client: Client,
}

struct PreparedHtmlInput {
    markup: String,
    search_paths: Vec<PathBuf>,
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            fonts: Mutex::new(Fonts::default()),
            search_paths: RwLock::new(Vec::new()),
            file_cache: Arc::new(Mutex::new(FileCache::default())),
            font_cache: Mutex::new(FontCache::default()),
            image_cache: Mutex::new(ImageCache::default()),
            #[cfg(feature = "http")]
            http_client: Client::new(),
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
        *self.fonts.lock().map_err(|_| state_lock_error("fonts"))? = Fonts::default();
        self.file_cache
            .lock()
            .map_err(|_| state_lock_error("file_cache"))?
            .clear();
        self.font_cache
            .lock()
            .map_err(|_| state_lock_error("font_cache"))?
            .clear();
        self.image_cache
            .lock()
            .map_err(|_| state_lock_error("image_cache"))?
            .clear();
        Ok(())
    }

    pub fn set_image_cache_enabled(&self, enabled: bool) -> Result<()> {
        self.image_cache
            .lock()
            .map_err(|_| state_lock_error("image_cache"))?
            .set_enabled(enabled);
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

    #[doc(hidden)]
    pub fn debug_image_cache_entries(&self) -> usize {
        self.image_cache
            .lock()
            .map(|cache| cache.entry_count())
            .unwrap_or_default()
    }

    #[doc(hidden)]
    pub fn debug_prune_image_cache(&self) -> Result<()> {
        let pass = self.next_image_cache_pass()?;
        self.prune_image_cache(pass)
    }

    fn render_request(&self, request: RenderRequest) -> Result<RenderedImage> {
        let pass = self.next_image_cache_pass()?;
        let html_result = self.prepare_html_result(&request)?;
        let fetched_resources = self.preload_fetched_resources(&html_result, pass)?;
        let stylesheet = html_result.stylesheet();
        let node = html_result.node;
        let fonts = self.fonts.lock().map_err(|_| state_lock_error("fonts"))?;
        let viewport = if request_has_auto_viewport(&request) {
            let measured = measure(
                RenderOptions::builder()
                    .viewport(make_viewport_from_request(&request))
                    .node(node.clone())
                    .images(fetched_resources.clone())
                    .stylesheet(stylesheet.clone())
                    .fonts(&fonts)
                    .build(),
            )?;
            let (width, height) = resolved_layout_size(&measured, request.viewport.clone());
            make_viewport(Some(width), Some(height), request.viewport.device_pixel_ratio)
        } else {
            make_viewport_from_request(&request)
        };
        let image = takumi_render(
            RenderOptions::builder()
                .viewport(viewport)
                .node(node)
                .images(fetched_resources)
                .stylesheet(stylesheet)
                .fonts(&fonts)
                .build(),
        )?;
        let bytes = self.encode_image(&image, &request)?;
        drop(fonts);
        self.prune_image_cache(pass)?;

        Ok(RenderedImage {
            bytes,
            format: request.format,
            width: image.width(),
            height: image.height(),
            content_type: Some(request.format.content_type().to_string()),
        })
    }

    fn measure_request(&self, request: RenderRequest) -> Result<MeasuredLayout> {
        let pass = self.next_image_cache_pass()?;
        let html_result = self.prepare_html_result(&request)?;
        let fetched_resources = self.preload_fetched_resources(&html_result, pass)?;
        let stylesheet = html_result.stylesheet();
        let node = html_result.node;
        let viewport = make_viewport_from_request(&request);
        let fonts = self.fonts.lock().map_err(|_| state_lock_error("fonts"))?;
        let measured = measure(
            RenderOptions::builder()
                .viewport(viewport)
                .node(node)
                .images(fetched_resources)
                .stylesheet(stylesheet)
                .fonts(&fonts)
                .build(),
        )?;
        let (width, height) = resolved_layout_size(&measured, request.viewport);
        drop(fonts);
        self.prune_image_cache(pass)?;

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
            .local_asset_mode(LocalAssetMode::AbsolutePath)
            .stylesheet_compiler(Box::new(compile_stylesheet));

        if let Some(emoji_type) = request.emoji_type {
            options = options.emoji_type(emoji_type.into_emoji_type());
        }

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
        pass: u64,
    ) -> Result<HashMap<Arc<str>, ImageSource>> {
        let mut fetched_resources = HashMap::new();

        for url in html_result.resource_urls() {
            self.maybe_insert_fetched_resource(&mut fetched_resources, url, pass)?;
        }

        Ok(fetched_resources)
    }

    fn maybe_insert_fetched_resource(
        &self,
        fetched_resources: &mut HashMap<Arc<str>, ImageSource>,
        raw_url: &str,
        pass: u64,
    ) -> Result<()> {
        if fetched_resources.contains_key(raw_url) {
            return Ok(());
        }

        if is_local_cached_asset_reference(raw_url) {
            let image = self.load_cached_image(raw_url, pass)?;
            fetched_resources.insert(Arc::<str>::from(raw_url), image);
        } else if cfg!(feature = "http") && is_remote_url(raw_url) {
            #[cfg(feature = "http")]
            {
                let image = self.fetch_remote_image(raw_url, pass)?;
                fetched_resources.insert(Arc::<str>::from(raw_url), image);
            }
        }

        Ok(())
    }

    #[cfg(feature = "http")]
    fn fetch_remote_image(&self, url: &str, pass: u64) -> Result<ImageSource> {
        let mut cache = self
            .image_cache
            .lock()
            .map_err(|_| state_lock_error("image_cache"))?;

        if cache.is_enabled() {
            let signature = FileSignature::remote();
            if let Some(image) = cache.get_auto(url, &signature, pass) {
                return Ok(image);
            }
        }
        drop(cache);

        let bytes = self
            .http_client
            .get(url)
            .send()
            .and_then(|resp| resp.error_for_status()?.bytes())
            .map_err(|error| RendererError::Render(format!("failed to fetch `{url}`: {error}")))?;

        let image = ImageSource::from_bytes(&bytes)
            .map_err(|error| RendererError::Render(error.to_string()))?;

        let mut cache = self
            .image_cache
            .lock()
            .map_err(|_| state_lock_error("image_cache"))?;
        if cache.is_enabled() {
            cache.insert(url.to_string(), FileSignature::remote(), image.clone(), pass);
        }
        Ok(image)
    }

    fn load_cached_image(&self, raw_url: &str, pass: u64) -> Result<ImageSource> {
        let (path_part, _) = split_reference_suffix(raw_url);
        let normalized = normalize_existing_path(&cached_asset_reference_path(path_part))?;
        let cache_key = normalized.to_string_lossy().replace('\\', "/");

        let mut cache = self
            .image_cache
            .lock()
            .map_err(|_| state_lock_error("image_cache"))?;

        if !cache.is_enabled() {
            drop(cache);
            return self.decode_image_file(&normalized);
        }

        let signature = FileSignature::from_path(&normalized)?;
        if let Some(image) = cache.get_auto(&cache_key, &signature, pass) {
            return Ok(image);
        }
        drop(cache);

        let image = self.decode_image_file(&normalized)?;
        self.image_cache
            .lock()
            .map_err(|_| state_lock_error("image_cache"))?
            .insert(cache_key, signature, image.clone(), pass);
        Ok(image)
    }

    fn decode_image_file(&self, normalized: &Path) -> Result<ImageSource> {
        let bytes = {
            let mut file_cache = self
                .file_cache
                .lock()
                .map_err(|_| state_lock_error("file_cache"))?;
            file_cache.read_bytes(normalized)?
        };
        ImageSource::from_bytes(&bytes)
            .map_err(|error| RendererError::Render(error.to_string()))
    }

    fn next_image_cache_pass(&self) -> Result<u64> {
        Ok(self
            .image_cache
            .lock()
            .map_err(|_| state_lock_error("image_cache"))?
            .next_pass())
    }

    fn prune_image_cache(&self, pass: u64) -> Result<()> {
        let mut image_cache = self
            .image_cache
            .lock()
            .map_err(|_| state_lock_error("image_cache"))?;
        if image_cache.is_enabled() {
            image_cache.prune(pass);
        }
        Ok(())
    }

    fn encode_image(&self, image: &Bitmap, request: &RenderRequest) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        write_image(
            image,
            &mut bytes,
            request.format.into_output_format(request.quality),
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

        self.fonts
            .lock()
            .map_err(|_| state_lock_error("fonts"))?
            .register(FontResource::new(&*bytes))?;

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

        self.fonts
            .lock()
            .map_err(|_| state_lock_error("fonts"))?
            .register(FontResource::new(bytes.as_slice()))?;

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

fn make_viewport(width: Option<u32>, height: Option<u32>, device_pixel_ratio: Option<f32>) -> Viewport {
    let mut viewport = Viewport::new((width, height));
    if let Some(dpr) = device_pixel_ratio {
        viewport = viewport.with_device_pixel_ratio(dpr);
    }
    viewport
}

fn make_viewport_from_request(request: &RenderRequest) -> Viewport {
    make_viewport(
        request.viewport.width,
        request.viewport.height,
        request.viewport.device_pixel_ratio,
    )
}

fn request_has_auto_viewport(request: &RenderRequest) -> bool {
    request.viewport.width.is_none() || request.viewport.height.is_none()
}

fn resolved_layout_size(measured: &MeasuredNode, viewport: RenderSize) -> (u32, u32) {
    let width = viewport
        .width
        .unwrap_or_else(|| measured.width.round().max(0.0) as u32);
    let height = viewport
        .height
        .unwrap_or_else(|| measured.height.round().max(0.0) as u32);
    (width, height)
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

fn is_remote_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
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

fn compile_stylesheet(source: &str, base_path: Option<&Path>) -> std::result::Result<String, HtmlError> {
    let mut opts = grass::Options::default();
    if let Some(dir) = base_path {
        opts = opts.load_path(dir);
    }
    grass::from_string(source, &opts).map_err(|error| HtmlError::StylesheetCompilationFailed {
        reason: error.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use std::{fs, time::Duration};

    use super::{Renderer, collect_font_files};
    use crate::api::{ImageFormat, RenderInput, RenderRequest, RenderSize};
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
            emoji_type: None,
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
    fn auto_image_cache_reloads_when_source_file_changes() {
        let temp = tempdir().expect("tempdir");
        let image_path = temp.path().join("relative.png");
        fs::write(&image_path, solid_png_bytes([255, 0, 0, 255])).expect("write red image");

        let renderer = Renderer::default();
        renderer
            .add_search_path(temp.path().to_string_lossy().into_owned())
            .expect("add search path");
        let request = inline_html_request(
            r#"<!doctype html><html><body><img src="relative.png" width="1" height="1"></body></html>"#,
        );

        let first = renderer
            .render(request.clone())
            .expect("render red local image");
        assert!(rendered_contains_pixel(&first.bytes, [255, 0, 0, 255]));

        std::thread::sleep(Duration::from_millis(20));
        fs::write(&image_path, solid_png_bytes([0, 0, 255, 255])).expect("write blue image");

        let second = renderer.render(request).expect("render blue local image");
        assert!(rendered_contains_pixel(&second.bytes, [0, 0, 255, 255]));
    }

    #[test]
    fn auto_image_cache_reuses_unchanged_images() {
        let temp = tempdir().expect("tempdir");
        fs::write(temp.path().join("relative.png"), tiny_png_bytes()).expect("write image");

        let renderer = Renderer::default();
        renderer
            .add_search_path(temp.path().to_string_lossy().into_owned())
            .expect("add search path");
        let request = inline_html_request(
            r#"<!doctype html><html><body><img src="relative.png" width="1" height="1"></body></html>"#,
        );

        renderer
            .render(request.clone())
            .expect("render local image first time");
        renderer
            .render(request)
            .expect("render local image second time");

        assert_eq!(renderer.debug_image_cache_entries(), 1);
    }

    #[test]
    fn disabled_image_cache_does_not_reuse_images_after_source_is_removed() {
        let temp = tempdir().expect("tempdir");
        let image_path = temp.path().join("relative.png");
        fs::write(&image_path, tiny_png_bytes()).expect("write image");

        let renderer = Renderer::default();
        renderer
            .set_image_cache_enabled(false)
            .expect("disable image cache");
        renderer
            .add_search_path(temp.path().to_string_lossy().into_owned())
            .expect("add search path");
        renderer
            .render(inline_html_request(
                r#"<!doctype html><html><body><img src="relative.png" width="1" height="1"></body></html>"#,
            ))
            .expect("render local image");

        assert_eq!(renderer.debug_image_cache_entries(), 0);

        let absolute_reference = normalize_existing_path(&image_path)
            .expect("normalize image path")
            .to_string_lossy()
            .replace('\\', "/");
        fs::remove_file(&image_path).expect("remove source image");
        let pass = renderer.next_image_cache_pass().expect("next pass");

        renderer
            .load_cached_image(&absolute_reference, pass)
            .expect_err("disabled cache should not reuse deleted source image");
    }

    #[test]
    fn auto_image_cache_prunes_inactive_images() {
        let temp = tempdir().expect("tempdir");
        fs::write(temp.path().join("relative.png"), tiny_png_bytes()).expect("write image");

        let renderer = Renderer::default();
        renderer
            .add_search_path(temp.path().to_string_lossy().into_owned())
            .expect("add search path");
        renderer
            .render(inline_html_request(
                r#"<!doctype html><html><body><img src="relative.png" width="1" height="1"></body></html>"#,
            ))
            .expect("render local image");
        assert_eq!(renderer.debug_image_cache_entries(), 1);

        for _ in 0..65 {
            renderer
                .debug_prune_image_cache()
                .expect("prune image cache");
        }

        assert_eq!(renderer.debug_image_cache_entries(), 0);
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
            emoji_type: None,
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
            emoji_type: None,
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
        let all_urls: Vec<&str> = html_result.resource_urls().collect();
        assert!(
            all_urls.contains(&expected.as_str()),
            "expected resolved stylesheet URL `{expected}` in resource_urls, got {all_urls:?}"
        );

        let preloaded = renderer
            .preload_fetched_resources(&html_result, 1)
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
            emoji_type: None,
        };

        let error = super::validate_request(&request).expect_err("reject zero width");
        assert!(error.to_string().contains("viewport width"));
    }

    #[test]
    fn emoji_type_replaces_emoji_with_cdn_images() {
        use crate::api::EmojiSet;

        let renderer = Renderer::default();
        let request = RenderRequest {
            input: RenderInput::Inline("<div>Hello 😀</div>".to_string()),
            viewport: RenderSize {
                width: Some(20),
                height: Some(20),
                device_pixel_ratio: None,
            },
            format: ImageFormat::Png,
            quality: None,
            load_linked_stylesheets: None,
            normalize_whitespace: None,
            emoji_type: Some(EmojiSet::Twemoji),
        };
        let prepared = renderer.resolve_markup_input(&request).unwrap();
        let html_result = renderer
            .convert_markup(&prepared.markup, &request, &prepared.search_paths)
            .unwrap();

        let urls: Vec<&str> = html_result.resource_urls().collect();
        assert!(
            urls.iter().any(|u| u.contains("1f600.svg")),
            "expected Twemoji URL for 😀 in resource_urls, got {urls:?}"
        );

        let rendered = html_result.node.to_html();
        assert!(
            rendered.contains("cdn.jsdelivr.net"),
            "expected CDN image in rendered HTML, got {rendered}"
        );
    }

    #[test]
    fn emoji_renders_from_local_svg() {
        // Emoji CDN URLs are remote and won't be fetched by preload_fetched_resources.
        // To actually render emoji images, the caller must pre-fetch them into the
        // images map. Here we simulate this by placing a local SVG that the renderer
        // can load through the search path mechanism.
        let temp = tempdir().expect("tempdir");
        let emoji_svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 36 36\"><circle cx=\"18\" cy=\"18\" r=\"16\" fill=\"#FF0000\"/></svg>";
        fs::write(temp.path().join("emoji.svg"), emoji_svg).expect("write emoji svg");

        let renderer = Renderer::default();
        renderer
            .add_search_path(temp.path().to_string_lossy().into_owned())
            .expect("add search path");

        // Render with a local SVG image to verify the SVG rendering pipeline works
        let request = RenderRequest {
            input: RenderInput::Inline(
                r#"<div style="width: 32px; height: 32px;"><img src="emoji.svg" width="32" height="32"></div>"#
                    .to_string(),
            ),
            viewport: RenderSize {
                width: Some(32),
                height: Some(32),
                device_pixel_ratio: None,
            },
            format: ImageFormat::Png,
            quality: None,
            load_linked_stylesheets: None,
            normalize_whitespace: None,
            emoji_type: None,
        };
        let rendered = renderer.render(request).expect("render local svg");

        let decoded = image::load_from_memory_with_format(&rendered.bytes, image::ImageFormat::Png)
            .expect("decode rendered png");
        let has_red_pixel = decoded
            .to_rgba8()
            .pixels()
            .any(|pixel| pixel.0[0] > 200 && pixel.0[1] < 50 && pixel.0[2] < 50);
        assert!(
            has_red_pixel,
            "expected red pixel from emoji SVG circle"
        );
    }

    #[test]
    fn no_emoji_type_preserves_emoji_as_text() {
        let renderer = Renderer::default();
        let request = RenderRequest {
            input: RenderInput::Inline("<div>Hello 😀</div>".to_string()),
            viewport: RenderSize {
                width: Some(20),
                height: Some(20),
                device_pixel_ratio: None,
            },
            format: ImageFormat::Png,
            quality: None,
            load_linked_stylesheets: None,
            normalize_whitespace: None,
            emoji_type: None,
        };
        let prepared = renderer.resolve_markup_input(&request).unwrap();
        let html_result = renderer
            .convert_markup(&prepared.markup, &request, &prepared.search_paths)
            .unwrap();

        let rendered = html_result.node.to_html();
        assert!(
            rendered.contains("😀"),
            "expected emoji preserved as text when emoji_type is None, got {rendered}"
        );
        assert!(
            !rendered.contains("cdn.jsdelivr.net"),
            "should not contain CDN URL when emoji_type is None"
        );
    }

    #[test]
    #[ignore] // requires network access
    fn emoji_renders_from_cdn() {
        use crate::api::EmojiSet;

        let renderer = Renderer::new();
        let request = RenderRequest {
            input: RenderInput::Inline("<div style=\"width: 32px; height: 32px;\">Hello 😀</div>".to_string()),
            viewport: RenderSize {
                width: Some(32),
                height: Some(32),
                device_pixel_ratio: None,
            },
            format: ImageFormat::Png,
            quality: None,
            load_linked_stylesheets: None,
            normalize_whitespace: None,
            emoji_type: Some(EmojiSet::Twemoji),
        };
        let rendered = renderer.render(request).expect("render emoji from CDN");

        let decoded = image::load_from_memory_with_format(&rendered.bytes, image::ImageFormat::Png)
            .expect("decode rendered png");
        // The emoji SVG contains colored pixels; verify the image is not blank.
        let has_non_white_pixel = decoded.to_rgba8().pixels().any(|p| {
            !(p.0[0] > 250 && p.0[1] > 250 && p.0[2] > 250)
        });
        assert!(
            has_non_white_pixel,
            "expected emoji pixels in rendered image"
        );
    }

    fn tiny_png_bytes() -> Vec<u8> {
        solid_png_bytes([255, 0, 0, 255])
    }

    fn solid_png_bytes(color: [u8; 4]) -> Vec<u8> {
        let image = image::RgbaImage::from_pixel(1, 1, image::Rgba(color));
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgba8(image)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .expect("encode tiny png");
        bytes
    }

    fn rendered_contains_pixel(bytes: &[u8], color: [u8; 4]) -> bool {
        image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
            .expect("decode rendered png")
            .to_rgba8()
            .pixels()
            .any(|pixel| pixel.0 == color)
    }
}
