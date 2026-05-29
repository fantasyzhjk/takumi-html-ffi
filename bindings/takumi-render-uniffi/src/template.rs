use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
};

use minijinja::Value as JinjaValue;
use minijinja::{AutoEscape, Environment, Error, ErrorKind};
use serde_json::Value;

use crate::{
    api::{InlineTemplateInput, TemplateRequest, TemplateContentKind, TemplateInput},
    cache::{FileCache, normalize_existing_path},
    error::{RendererError, Result},
    markdown::{self, FormattingConfig, render_markdown_to_html},
};

#[derive(uniffi::Object)]
pub struct TemplateEngine {
    search_paths: RwLock<Vec<PathBuf>>,
    registered_templates: RwLock<HashMap<String, String>>,
    file_cache: Arc<Mutex<FileCache>>,
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self {
            search_paths: RwLock::new(Vec::new()),
            registered_templates: RwLock::new(HashMap::new()),
            file_cache: Arc::new(Mutex::new(FileCache::default())),
        }
    }
}

#[uniffi::export]
impl TemplateEngine {
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
            return Err(RendererError::invalid_request(
                "template name cannot be empty",
            ));
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

    pub fn render(&self, request: TemplateRequest) -> Result<String> {
        self.render_request(request)
    }
}

impl TemplateEngine {
    fn render_request(&self, request: TemplateRequest) -> Result<String> {
        validate_render_request(&request)?;

        let repository = self.template_repository()?;
        let resolved = resolve_source(&request, &repository)?;
        let markup = if resolved.requires_jinja() {
            render_template_markup(&resolved, request.context_json.as_deref(), &repository)?
        } else {
            resolved.source_text.clone()
        };

        if resolved.requires_markdown() {
            render_markdown_to_html(&markup, &resolved.formatting)
        } else {
            Ok(markup)
        }
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
}

#[derive(Debug, Clone)]
pub(crate) struct TemplateRepository {
    pub search_paths: Vec<PathBuf>,
    pub registered_templates: HashMap<String, String>,
    pub file_cache: Arc<Mutex<FileCache>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TemplateSourceKind {
    Inline,
    File,
    Registered,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedSource {
    pub content_kind: TemplateContentKind,
    source_kind: TemplateSourceKind,
    pub source_text: String,
    pub logical_name: String,
    pub search_paths: Vec<PathBuf>,
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
        return Err(RendererError::invalid_request(
            "search path cannot be empty",
        ));
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

pub(crate) fn resolve_source(
    request: &TemplateRequest,
    repository: &TemplateRepository,
) -> Result<ResolvedSource> {
    let formatting = FormattingConfig::new(request.syntax_theme.as_deref())?;
    let search_paths = repository.search_paths.clone();

    match &request.input {
        TemplateInput::Inline(input) => {
            resolve_inline_source(input, request.content_kind, search_paths, formatting)
        }
        TemplateInput::File(path) => resolve_file_source(
            path,
            request.content_kind,
            repository,
            search_paths,
            formatting,
        ),
        TemplateInput::Registered(name) => resolve_registered_source(
            name,
            request.content_kind,
            repository,
            search_paths,
            formatting,
        ),
    }
}

pub(crate) fn render_template_markup(
    resolved: &ResolvedSource,
    context_json: Option<&str>,
    repository: &TemplateRepository,
) -> Result<String> {
    let context = parse_context_json(context_json)?;
    let environment = build_environment(repository, resolved)?;
    let template = environment.get_template(&resolved.logical_name)?;
    Ok(template.render(context)?)
}

fn validate_render_request(request: &TemplateRequest) -> Result<()> {
    validate_template_input(&request.input)?;
    let _ = FormattingConfig::new(request.syntax_theme.as_deref())?;
    Ok(())
}

fn validate_template_input(input: &TemplateInput) -> Result<()> {
    match input {
        TemplateInput::Inline(input) => {
            if input
                .logical_name
                .as_deref()
                .is_some_and(|name| name.trim().is_empty())
            {
                return Err(RendererError::invalid_request(
                    "input.logical_name cannot be empty when provided",
                ));
            }
        }
        TemplateInput::File(path) => {
            if path.trim().is_empty() {
                return Err(RendererError::invalid_request("input.file cannot be empty"));
            }
        }
        TemplateInput::Registered(name) => {
            if name.trim().is_empty() {
                return Err(RendererError::invalid_request(
                    "input.registered cannot be empty",
                ));
            }
        }
    }

    Ok(())
}

fn resolve_inline_source(
    input: &InlineTemplateInput,
    content_kind: TemplateContentKind,
    search_paths: Vec<PathBuf>,
    formatting: FormattingConfig,
) -> Result<ResolvedSource> {
    let logical_name = input
        .logical_name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| default_inline_logical_name(content_kind).to_string());

    Ok(ResolvedSource {
        content_kind,
        source_kind: TemplateSourceKind::Inline,
        source_text: input.source.clone(),
        logical_name,
        search_paths,
        formatting,
    })
}

fn resolve_file_source(
    reference: &str,
    content_kind: TemplateContentKind,
    repository: &TemplateRepository,
    search_paths: Vec<PathBuf>,
    formatting: FormattingConfig,
) -> Result<ResolvedSource> {
    let path = resolve_existing_template_path(reference.trim(), &search_paths)?
        .ok_or_else(|| RendererError::template_not_found(reference))?;
    let source_text = {
        let mut cache = repository
            .file_cache
            .lock()
            .map_err(|_| RendererError::Render("template cache lock poisoned".to_string()))?;
        cache.read_string(&path)?
    };

    Ok(ResolvedSource {
        content_kind,
        source_kind: TemplateSourceKind::File,
        source_text,
        logical_name: path.to_string_lossy().into_owned(),
        search_paths,
        formatting,
    })
}

fn resolve_registered_source(
    name: &str,
    content_kind: TemplateContentKind,
    repository: &TemplateRepository,
    search_paths: Vec<PathBuf>,
    formatting: FormattingConfig,
) -> Result<ResolvedSource> {
    let name = name.trim();
    let source_text = repository
        .registered_templates
        .get(name)
        .cloned()
        .ok_or_else(|| RendererError::template_not_found(name))?;

    Ok(ResolvedSource {
        content_kind,
        source_kind: TemplateSourceKind::Registered,
        source_text,
        logical_name: name.to_string(),
        search_paths,
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
    let escape_html = matches!(resolved.content_kind, TemplateContentKind::JinjaHtml);

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
                parent_path
                    .parent()
                    .unwrap_or_else(|| Path::new(""))
                    .join(name)
            } else {
                return Cow::Borrowed(name);
            };

            Cow::Owned(
                normalize_template_path(&joined)
                    .to_string_lossy()
                    .into_owned(),
            )
        }
    });

    for (name, source) in registered_templates {
        env.add_template_owned(name, source)?;
    }

    if resolved.source_kind != TemplateSourceKind::Registered {
        env.add_template_owned(resolved.logical_name.clone(), resolved.source_text.clone())?;
    }

    let markdown_formatting = resolved.formatting.clone();
    env.add_filter("markdown", move |value: String| {
        markdown_filter(value, &markdown_formatting)
    });
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

fn resolve_existing_template_path(
    reference: &str,
    search_paths: &[PathBuf],
) -> std::io::Result<Option<PathBuf>> {
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

fn parse_context_json(context_json: Option<&str>) -> Result<Value> {
    let trimmed = context_json.map(str::trim).unwrap_or_default();
    if trimmed.is_empty() {
        Ok(Value::Object(Default::default()))
    } else {
        Ok(serde_json::from_str(trimmed)?)
    }
}

fn default_inline_logical_name(content_kind: TemplateContentKind) -> &'static str {
    match content_kind {
        TemplateContentKind::Markdown => "inline.md",
        TemplateContentKind::JinjaHtml => "inline.html.jinja",
        TemplateContentKind::JinjaMarkdown => "inline.md.jinja",
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

fn markdown_filter(
    value: String,
    formatting: &FormattingConfig,
) -> std::result::Result<JinjaValue, Error> {
    let html =
        markdown::render_markdown_to_html(&value, formatting).map_err(template_filter_error)?;
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

fn state_lock_error(name: &str) -> RendererError {
    RendererError::Render(format!("template engine state lock `{name}` was poisoned"))
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        fs,
        sync::{Arc, Mutex},
    };

    use tempfile::TempDir;

    use super::*;
    use crate::{api::TemplateRequest, cache::FileCache};

    fn render_inline(
        content_kind: TemplateContentKind,
        template_source: &str,
        context_json: Option<&str>,
    ) -> String {
        let engine = TemplateEngine::default();
        let request = TemplateRequest {
            input: TemplateInput::Inline(InlineTemplateInput {
                source: template_source.to_string(),
                logical_name: Some("inline/index.jinja".to_string()),
            }),
            context_json: context_json.map(ToOwned::to_owned),
            content_kind,
            syntax_theme: Some("base16-ocean.dark".to_string()),
        };

        engine
            .render_request(request)
            .expect("render inline template")
    }

    #[test]
    fn renders_nested_json_from_inline_template() {
        let rendered = render_inline(
            TemplateContentKind::JinjaHtml,
            "<div>{{ user.profile.display_name }}</div>",
            Some(r#"{"user":{"profile":{"display_name":"Takumi"}}}"#),
        );

        assert_eq!(rendered, "<div>Takumi</div>");
    }

    #[test]
    fn renders_static_inline_template_without_context_json() {
        let rendered = render_inline(TemplateContentKind::JinjaHtml, "<div>Takumi</div>", None);

        assert_eq!(rendered, "<div>Takumi</div>");
    }

    #[test]
    fn renders_plain_markdown_without_template_state() {
        let rendered = render_inline(
            TemplateContentKind::Markdown,
            "# Title\n\nHello *world*.",
            None,
        );

        assert_eq!(rendered, "<h1>Title</h1>\n<p>Hello <em>world</em>.</p>\n");
    }

    #[test]
    fn parse_context_json_treats_missing_and_blank_as_empty_object() {
        assert_eq!(
            parse_context_json(None).expect("parse missing context"),
            Value::Object(Default::default())
        );
        assert_eq!(
            parse_context_json(Some("   \n\t  ")).expect("parse blank context"),
            Value::Object(Default::default())
        );
    }

    #[test]
    fn resolves_relative_includes_from_template_file_directory() {
        let temp = TempDir::new().expect("tempdir");
        let template_dir = temp.path().join("templates");
        fs::create_dir_all(template_dir.join("partials")).expect("create template dir");
        fs::write(
            template_dir.join("index.jinja"),
            "<div>{% include './partials/footer.jinja' %}</div>",
        )
        .expect("write index template");
        fs::write(template_dir.join("partials/footer.jinja"), "Footer")
            .expect("write footer template");

        let engine = TemplateEngine::default();
        engine
            .add_search_path(template_dir.to_string_lossy().into_owned())
            .expect("add search path");
        let rendered = engine
            .render(TemplateRequest {
                input: TemplateInput::File("index.jinja".to_string()),
                context_json: None,
                content_kind: TemplateContentKind::JinjaHtml,
                syntax_theme: None,
            })
            .expect("render file template");
        assert_eq!(rendered, "<div>Footer</div>");
    }

    #[test]
    fn renders_markdown_filter() {
        let rendered = render_inline(
            TemplateContentKind::JinjaHtml,
            "{{ content | markdown }}",
            Some(r##"{"content":"# Title\n\nHello *world*."}"##),
        );

        assert_eq!(rendered, "<h1>Title</h1>\n<p>Hello <em>world</em>.</p>\n");
    }

    #[test]
    fn renders_datetime_format_filter() {
        let rendered = render_inline(
            TemplateContentKind::JinjaHtml,
            "{{ ts | datetime_format(\"%Y-%m-%d %H:%M:%S\") }}",
            Some(r#"{"ts":0}"#),
        );

        assert_eq!(rendered, "1970-01-01 00:00:00");
    }

    #[test]
    fn renders_filesize_filter() {
        let rendered = render_inline(
            TemplateContentKind::JinjaHtml,
            "{{ bytes | filesize }}",
            Some(r#"{"bytes":1536}"#),
        );

        assert_eq!(rendered, "1.50 KB");
    }

    #[test]
    fn renders_to_hex_filter() {
        let rendered = render_inline(
            TemplateContentKind::JinjaHtml,
            "{{ value | to_hex(width=2) }}",
            Some(r#"{"value":255}"#),
        );

        assert_eq!(rendered, "0xFF");
    }

    #[test]
    fn renders_json_pretty_filter() {
        let rendered = render_inline(
            TemplateContentKind::JinjaHtml,
            "{{ data | json_pretty }}",
            Some(r#"{"data":{"name":"Takumi","enabled":true}}"#),
        );

        assert!(rendered.starts_with("{\n  "));
        assert!(rendered.contains("\"name\": \"Takumi\""));
        assert!(rendered.contains("\"enabled\": true"));
        assert!(rendered.ends_with("\n}"));
    }

    #[test]
    fn renders_highlight_filter() {
        let rendered = render_inline(
            TemplateContentKind::JinjaHtml,
            "{{ code | highlight(\"rust\") }}",
            Some(r#"{"code":"let x = 1;"}"#),
        );

        assert!(rendered.starts_with("<pre><code>"));
        assert!(rendered.ends_with("</code></pre>"));
        assert!(rendered.contains("let"));
        assert!(rendered.contains("x"));
        assert!(rendered.contains("1"));
    }

    #[test]
    fn resolve_source_supports_registered_templates() {
        let repository = TemplateRepository {
            search_paths: Vec::new(),
            registered_templates: HashMap::from([(
                "cards/profile.jinja".to_string(),
                "<div>{{ name }}</div>".to_string(),
            )]),
            file_cache: Arc::new(Mutex::new(FileCache::default())),
        };

        let request = TemplateRequest {
            input: TemplateInput::Registered("cards/profile.jinja".to_string()),
            context_json: None,
            content_kind: TemplateContentKind::JinjaHtml,
            syntax_theme: None,
        };

        let resolved = resolve_source(&request, &repository).expect("resolve source");
        assert_eq!(resolved.logical_name, "cards/profile.jinja");
        assert_eq!(resolved.source_text, "<div>{{ name }}</div>");
    }
}
