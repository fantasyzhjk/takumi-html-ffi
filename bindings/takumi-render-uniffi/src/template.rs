use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex},
};

use minijinja::Value as JinjaValue;
use minijinja::{AutoEscape, Environment, Error, ErrorKind};
use serde_json::Value;

use crate::{
    api::{RenderContentKind, RenderInput, RenderRequest, RenderSourceKind},
    cache::{FileCache, normalize_existing_path},
    error::{RendererError, Result},
    markdown::{self, FormattingConfig},
};

#[derive(Debug, Clone)]
pub(crate) struct TemplateRepository {
    pub search_paths: Vec<PathBuf>,
    pub registered_templates: HashMap<String, String>,
    pub file_cache: Arc<Mutex<FileCache>>,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedSource {
    pub content_kind: RenderContentKind,
    pub source_kind: RenderSourceKind,
    pub source_text: String,
    pub logical_name: String,
    pub search_paths: Vec<PathBuf>,
    pub base_candidates: Vec<PathBuf>,
    pub formatting: FormattingConfig,
}

impl ResolvedSource {
    pub(crate) fn requires_jinja(&self) -> bool {
        self.content_kind.requires_jinja()
    }

    pub(crate) fn requires_markdown(&self) -> bool {
        self.content_kind.requires_markdown()
    }
}

pub(crate) fn normalize_search_path(path: &str) -> Result<PathBuf> {
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

pub(crate) fn validate_render_input(input: &RenderInput) -> Result<()> {
    if input.logical_name.as_deref().is_some_and(|name| name.trim().is_empty()) {
        return Err(RendererError::invalid_request("input.logical_name cannot be empty when provided"));
    }

    if input.base_path.as_deref().is_some_and(|path| path.trim().is_empty()) {
        return Err(RendererError::invalid_request("input.base_path cannot be empty when provided"));
    }

    if matches!(input.source_kind, RenderSourceKind::File | RenderSourceKind::Registered)
        && input.value.trim().is_empty()
    {
        return Err(RendererError::invalid_request(
            "input.value cannot be empty for file or registered sources",
        ));
    }

    if matches!(input.source_kind, RenderSourceKind::File | RenderSourceKind::Registered)
        && input.logical_name.is_some()
    {
        return Err(RendererError::invalid_request(
            "input.logical_name is only supported for inline sources",
        ));
    }

    if matches!(input.source_kind, RenderSourceKind::File | RenderSourceKind::Registered)
        && input.base_path.is_some()
    {
        return Err(RendererError::invalid_request(
            "input.base_path is only supported for inline sources",
        ));
    }

    if matches!(input.content_kind, RenderContentKind::Html) && input.syntax_theme.is_some() {
        return Err(RendererError::invalid_request(
            "input.syntax_theme is only supported for Markdown and Jinja content",
        ));
    }

    let _ = normalize_requested_search_paths(input.search_paths.as_deref())?;
    let _ = normalize_optional_base_path(input.base_path.as_deref())?;
    Ok(())
}

pub(crate) fn resolve_source(request: &RenderRequest, repository: &TemplateRepository) -> Result<ResolvedSource> {
    let formatting = FormattingConfig::from_input(&request.input)?;
    let requested_search_paths = normalize_requested_search_paths(request.input.search_paths.as_deref())?;
    let effective_search_paths = requested_search_paths
        .clone()
        .unwrap_or_else(|| repository.search_paths.clone());

    match request.input.source_kind {
        RenderSourceKind::Inline => resolve_inline_source(&request.input, effective_search_paths, formatting),
        RenderSourceKind::File => resolve_file_source(
            &request.input,
            repository,
            requested_search_paths,
            effective_search_paths,
            formatting,
        ),
        RenderSourceKind::Registered => resolve_registered_source(
            &request.input,
            repository,
            requested_search_paths,
            effective_search_paths,
            formatting,
        ),
    }
}

pub(crate) fn render_template_markup(
    resolved: &ResolvedSource,
    context_json: &str,
    repository: &TemplateRepository,
) -> Result<String> {
    let context = parse_context_json(context_json)?;
    let environment = build_environment(repository, resolved)?;
    let template = environment.get_template(&resolved.logical_name)?;
    Ok(template.render(context)?)
}

fn resolve_inline_source(
    input: &RenderInput,
    search_paths: Vec<PathBuf>,
    formatting: FormattingConfig,
) -> Result<ResolvedSource> {
    let logical_name = input
        .logical_name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| default_inline_logical_name(input.content_kind).to_string());
    let base_path = normalize_optional_base_path(input.base_path.as_deref())?;
    let base_candidates = collect_base_candidates(base_path.as_deref(), &search_paths);

    Ok(ResolvedSource {
        content_kind: input.content_kind,
        source_kind: input.source_kind,
        source_text: input.value.clone(),
        logical_name,
        search_paths,
        base_candidates,
        formatting,
    })
}

fn resolve_file_source(
    input: &RenderInput,
    repository: &TemplateRepository,
    requested_search_paths: Option<Vec<PathBuf>>,
    search_paths: Vec<PathBuf>,
    formatting: FormattingConfig,
) -> Result<ResolvedSource> {
    let reference = input.value.trim();
    let path = resolve_existing_template_path(reference, &search_paths)?
        .ok_or_else(|| RendererError::template_not_found(reference))?;
    let source_text = {
        let mut cache = repository
            .file_cache
            .lock()
            .map_err(|_| RendererError::Render("template cache lock poisoned".to_string()))?;
        cache.read_string(&path)?
    };

    let base_candidates = match requested_search_paths {
        Some(paths) => paths,
        None => collect_base_candidates(path.parent(), &repository.search_paths),
    };

    Ok(ResolvedSource {
        content_kind: input.content_kind,
        source_kind: input.source_kind,
        source_text,
        logical_name: path.to_string_lossy().into_owned(),
        search_paths,
        base_candidates,
        formatting,
    })
}

fn resolve_registered_source(
    input: &RenderInput,
    repository: &TemplateRepository,
    requested_search_paths: Option<Vec<PathBuf>>,
    search_paths: Vec<PathBuf>,
    formatting: FormattingConfig,
) -> Result<ResolvedSource> {
    let name = input.value.trim();
    let source_text = repository
        .registered_templates
        .get(name)
        .cloned()
        .ok_or_else(|| RendererError::template_not_found(name))?;

    let base_candidates = match requested_search_paths {
        Some(paths) => paths,
        None => collect_registered_base_candidates(name, &repository.search_paths),
    };

    Ok(ResolvedSource {
        content_kind: input.content_kind,
        source_kind: input.source_kind,
        source_text,
        logical_name: name.to_string(),
        search_paths,
        base_candidates,
        formatting,
    })
}

fn build_environment(
    repository: &TemplateRepository,
    resolved: &ResolvedSource,
) -> Result<Environment<'static>> {
    let registered_templates = repository.registered_templates.clone();
    let registered_names = Arc::new(
        registered_templates
            .keys()
            .cloned()
            .collect::<HashSet<String>>(),
    );
    let search_paths = resolved.search_paths.clone();
    let file_cache = Arc::clone(&repository.file_cache);
    let escape_html = matches!(resolved.content_kind, RenderContentKind::JinjaHtml);

    let mut env = Environment::new();
    env.set_auto_escape_callback(move |_| {
        if escape_html {
            AutoEscape::Html
        } else {
            AutoEscape::None
        }
    });
    env.set_path_join_callback({
        let registered_names = Arc::clone(&registered_names);
        move |name, parent| {
            if registered_names.contains(name) || Path::new(name).is_absolute() {
                return Cow::Borrowed(name);
            }

            let parent_path = Path::new(parent);
            let joined = if parent_path.is_absolute() {
                parent_path.parent().unwrap_or(parent_path).join(name)
            } else if parent.contains('/') || parent.contains('\\') {
                parent_path.parent().unwrap_or_else(|| Path::new("")).join(name)
            } else {
                return Cow::Borrowed(name);
            };

            Cow::Owned(normalize_template_path(&joined).to_string_lossy().into_owned())
        }
    });

    for (name, source) in registered_templates {
        env.add_template_owned(name, source)?;
    }

    if resolved.source_kind != RenderSourceKind::Registered {
        env.add_template_owned(resolved.logical_name.clone(), resolved.source_text.clone())?;
    }

    let markdown_formatting = resolved.formatting.clone();
    env.add_filter("markdown", move |value: String| markdown_filter(value, &markdown_formatting));
    env.add_filter("datetime_format", datetime_format_filter);
    env.add_filter("filesize", filesize_filter);
    env.add_filter("json_pretty", json_pretty_filter);
    env.add_filter("to_hex", to_hex_filter);
    let highlight_formatting = resolved.formatting.clone();
    env.add_filter("highlight", move |code: String, lang: String| {
        highlight_filter(code, lang, &highlight_formatting)
    });

    env.set_loader(move |name| load_template_source(name, &search_paths, &file_cache));
    Ok(env)
}

fn load_template_source(
    name: &str,
    search_paths: &[PathBuf],
    file_cache: &Arc<Mutex<FileCache>>,
) -> std::result::Result<Option<String>, Error> {
    let path = match resolve_existing_template_path(name, search_paths) {
        Ok(Some(path)) => path,
        Ok(None) => return Ok(None),
        Err(error) => return Err(loader_error(name, error)),
    };

    let mut cache = file_cache
        .lock()
        .map_err(|_| loader_error(name, "template cache lock poisoned"))?;
    cache
        .read_string(&path)
        .map(Some)
        .map_err(|error| loader_error(name, error))
}

fn loader_error(name: &str, error: impl std::fmt::Display) -> Error {
    Error::new(
        ErrorKind::InvalidOperation,
        format!("failed to load template `{name}`: {error}"),
    )
}

fn resolve_existing_template_path(reference: &str, search_paths: &[PathBuf]) -> std::io::Result<Option<PathBuf>> {
    let path = Path::new(reference);

    if path.is_absolute() {
        return if path.is_file() {
            Ok(Some(path.canonicalize()?))
        } else {
            Ok(None)
        };
    }

    if path.is_file() {
        return Ok(Some(path.canonicalize()?));
    }

    for search_path in search_paths {
        let candidate = search_path.join(path);
        if candidate.is_file() {
            return Ok(Some(candidate.canonicalize()?));
        }
    }

    Ok(None)
}

fn parse_context_json(context_json: &str) -> Result<Value> {
    let trimmed = context_json.trim();
    if trimmed.is_empty() {
        Ok(Value::Object(Default::default()))
    } else {
        Ok(serde_json::from_str(trimmed)?)
    }
}

fn normalize_requested_search_paths(search_paths: Option<&[String]>) -> Result<Option<Vec<PathBuf>>> {
    match search_paths {
        Some(paths) => paths
            .iter()
            .map(|path| normalize_search_path(path))
            .collect::<Result<Vec<_>>>()
            .map(Some),
        None => Ok(None),
    }
}

fn normalize_optional_base_path(base_path: Option<&str>) -> Result<Option<PathBuf>> {
    let Some(base_path) = base_path else {
        return Ok(None);
    };

    let normalized = normalize_existing_path(Path::new(base_path.trim()))?;
    if !normalized.is_dir() {
        return Err(RendererError::invalid_request(format!(
            "input.base_path `{}` is not a directory",
            normalized.display()
        )));
    }

    Ok(Some(normalized))
}

fn collect_base_candidates(primary: Option<&Path>, search_paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut base_candidates = Vec::new();

    if let Some(primary) = primary {
        push_unique_path(&mut base_candidates, primary.to_path_buf());
    }

    for path in search_paths {
        push_unique_path(&mut base_candidates, path.clone());
    }

    base_candidates
}

fn collect_registered_base_candidates(logical_name: &str, search_paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut base_candidates = Vec::new();
    let logical_parent = Path::new(logical_name)
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());

    if let Some(logical_parent) = logical_parent {
        for search_path in search_paths {
            push_unique_path(&mut base_candidates, search_path.join(logical_parent));
        }
    }

    for search_path in search_paths {
        push_unique_path(&mut base_candidates, search_path.clone());
    }

    base_candidates
}

fn push_unique_path(paths: &mut Vec<PathBuf>, candidate: PathBuf) {
    if !paths.iter().any(|existing| existing == &candidate) {
        paths.push(candidate);
    }
}

fn default_inline_logical_name(content_kind: RenderContentKind) -> &'static str {
    match content_kind {
        RenderContentKind::Html => "inline.html",
        RenderContentKind::Markdown => "inline.md",
        RenderContentKind::JinjaHtml => "inline.html.jinja",
        RenderContentKind::JinjaMarkdown => "inline.md.jinja",
    }
}

fn normalize_template_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(Path::new("/")),
        }
    }

    normalized
}

fn markdown_filter(value: String, formatting: &FormattingConfig) -> std::result::Result<JinjaValue, Error> {
    let html = markdown::render_markdown_to_html(&value, formatting).map_err(template_filter_error)?;
    Ok(JinjaValue::from_safe_string(html))
}

fn datetime_format_filter(ts: i64, format_str: &str) -> std::result::Result<String, Error> {
    use chrono::{DateTime, TimeZone, Utc};

    let dt = if ts.abs() > 9_999_999_999 {
        DateTime::from_timestamp_millis(ts)
    } else {
        Utc.timestamp_opt(ts, 0).single()
    };

    let dt = dt.ok_or_else(|| Error::new(ErrorKind::InvalidOperation, "非法的时间戳数据"))?;

    Ok(dt.format(format_str).to_string())
}

fn filesize_filter(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn json_pretty_filter(value: JinjaValue) -> std::result::Result<JinjaValue, Error> {
    let json = serde_json::to_string_pretty(&value)
        .map_err(|error| Error::new(ErrorKind::InvalidOperation, error.to_string()))?;
    Ok(JinjaValue::from_safe_string(json))
}

fn to_hex_filter(val: u64, kwargs: minijinja::value::Kwargs) -> std::result::Result<String, Error> {
    let width: usize = kwargs.get("width").unwrap_or(4);
    kwargs.assert_all_used()?;

    Ok(format!("0x{:0>width$X}", val, width = width))
}

fn highlight_filter(
    code: String,
    lang: String,
    formatting: &FormattingConfig,
) -> std::result::Result<JinjaValue, Error> {
    let html = markdown::highlight_code(&code, &lang, formatting).map_err(template_filter_error)?;
    Ok(JinjaValue::from_safe_string(html))
}

fn template_filter_error(error: RendererError) -> Error {
    Error::new(ErrorKind::InvalidOperation, error.to_string())
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::{Arc, Mutex}};

    use tempfile::TempDir;

    use super::*;
    use crate::{
        api::{
            ImageFormat,
            RenderContentKind,
            RenderInput,
            RenderRequest,
            RenderSize,
            RenderSourceKind,
        },
        cache::FileCache,
    };

    fn render_inline(template_source: &str, context_json: &str) -> String {
        let request = RenderRequest {
            input: RenderInput {
                source_kind: RenderSourceKind::Inline,
                content_kind: RenderContentKind::JinjaHtml,
                value: template_source.to_string(),
                logical_name: Some("inline.jinja".to_string()),
                base_path: None,
                search_paths: None,
                syntax_theme: None,
            },
            context_json: context_json.to_string(),
            viewport: RenderSize { width: 320, height: 120 },
            format: ImageFormat::Png,
            quality: None,
            load_linked_stylesheets: None,
            resolve_local_assets: None,
            normalize_whitespace: None,
        };
        let repository = TemplateRepository {
            search_paths: Vec::new(),
            registered_templates: HashMap::new(),
            file_cache: Arc::new(Mutex::new(FileCache::default())),
        };

        let resolved = resolve_source(&request, &repository).expect("resolve inline template");
        render_template_markup(&resolved, &request.context_json, &repository)
            .expect("render inline template")
    }

    #[test]
    fn renders_nested_json_from_inline_template() {
        let rendered = render_inline(
            "<div>{{ user.profile.display_name }}</div>",
            r#"{"user":{"profile":{"display_name":"Takumi"}}}"#,
        );

        assert_eq!(rendered, "<div>Takumi</div>");
    }

    #[test]
    fn resolves_relative_includes_from_template_file_directory() {
        let temp = TempDir::new().expect("tempdir");
        let template_dir = temp.path().join("templates");
        std::fs::create_dir_all(template_dir.join("partials")).expect("create template dir");
        std::fs::write(
            template_dir.join("index.jinja"),
            "<div>{% include './partials/footer.jinja' %}</div>",
        )
        .expect("write index template");
        std::fs::write(template_dir.join("partials/footer.jinja"), "Footer")
            .expect("write footer template");

        let request = RenderRequest {
            input: RenderInput {
                source_kind: RenderSourceKind::File,
                content_kind: RenderContentKind::JinjaHtml,
                value: template_dir.join("index.jinja").to_string_lossy().into_owned(),
                logical_name: None,
                base_path: None,
                search_paths: None,
                syntax_theme: None,
            },
            context_json: "{}".to_string(),
            viewport: RenderSize { width: 320, height: 120 },
            format: ImageFormat::Png,
            quality: None,
            load_linked_stylesheets: None,
            resolve_local_assets: None,
            normalize_whitespace: None,
        };
        let repository = TemplateRepository {
            search_paths: Vec::new(),
            registered_templates: HashMap::new(),
            file_cache: Arc::new(Mutex::new(FileCache::default())),
        };

        let resolved = resolve_source(&request, &repository).expect("resolve file template");
        let rendered = render_template_markup(&resolved, &request.context_json, &repository)
            .expect("render file template");
        assert_eq!(rendered, "<div>Footer</div>");
    }

    #[test]
    fn renders_markdown_filter() {
        let rendered = render_inline(
            "{{ content | markdown }}",
            r##"{"content":"# Title\n\nHello *world*."}"##,
        );

        assert_eq!(
            rendered,
            "<h1>Title</h1>\n<p>Hello <em>world</em>.</p>\n"
        );
    }

    #[test]
    fn renders_datetime_format_filter() {
        let rendered = render_inline(
            "{{ ts | datetime_format(\"%Y-%m-%d %H:%M:%S\") }}",
            r#"{"ts":0}"#,
        );

        assert_eq!(rendered, "1970-01-01 00:00:00");
    }

    #[test]
    fn renders_filesize_filter() {
        let rendered = render_inline(
            "{{ bytes | filesize }}",
            r#"{"bytes":1536}"#,
        );

        assert_eq!(rendered, "1.50 KB");
    }

    #[test]
    fn renders_to_hex_filter() {
        let rendered = render_inline(
            "{{ value | to_hex(width=2) }}",
            r#"{"value":255}"#,
        );

        assert_eq!(rendered, "0xFF");
    }

    #[test]
    fn renders_json_pretty_filter() {
        let rendered = render_inline(
            "{{ data | json_pretty }}",
            r#"{"data":{"name":"Takumi","enabled":true}}"#,
        );

        assert!(rendered.starts_with("{\n  "));
        assert!(rendered.contains("\"name\": \"Takumi\""));
        assert!(rendered.contains("\"enabled\": true"));
        assert!(rendered.ends_with("\n}"));
    }

    #[test]
    fn renders_highlight_filter() {
        let rendered = render_inline(
            "{{ code | highlight(\"rust\") }}",
            r#"{"code":"let x = 1;"}"#,
        );

        assert!(rendered.starts_with("<pre><code>"));
        assert!(rendered.ends_with("</code></pre>"));
        assert!(rendered.contains("let"));
        assert!(rendered.contains("x"));
        assert!(rendered.contains("1"));
    }

    #[test]
    fn resolve_source_uses_registered_logical_parent_before_renderer_search_paths() {
        let temp = TempDir::new().expect("tempdir");
        std::fs::create_dir_all(temp.path().join("cards")).expect("create cards dir");

        let repository = TemplateRepository {
            search_paths: vec![temp.path().to_path_buf()],
            registered_templates: HashMap::from([(
                "cards/profile.jinja".to_string(),
                "<img src=\"pixel.png\" />".to_string(),
            )]),
            file_cache: Arc::new(Mutex::new(FileCache::default())),
        };

        let request = RenderRequest {
            input: RenderInput {
                source_kind: RenderSourceKind::Registered,
                content_kind: RenderContentKind::JinjaHtml,
                value: "cards/profile.jinja".to_string(),
                logical_name: None,
                base_path: None,
                search_paths: None,
                syntax_theme: None,
            },
            context_json: "{}".to_string(),
            viewport: RenderSize { width: 320, height: 120 },
            format: ImageFormat::Png,
            quality: None,
            load_linked_stylesheets: None,
            resolve_local_assets: None,
            normalize_whitespace: None,
        };

        let resolved = resolve_source(&request, &repository).expect("resolve source");
        assert_eq!(resolved.base_candidates[0], temp.path().join("cards"));
    }
}
