use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex},
};

use minijinja::{AutoEscape, Environment, Error, ErrorKind};
use pulldown_cmark::{Options, Parser, html};
use serde_json::Value;
use minijinja::Value as JinjaValue;

use crate::{
    api::RenderRequest,
    cache::FileCache,
    error::{RendererError, Result},
};

#[derive(Debug, Clone)]
pub(crate) struct TemplateRepository {
    pub search_paths: Vec<PathBuf>,
    pub registered_templates: HashMap<String, String>,
    pub file_cache: Arc<Mutex<FileCache>>,
}

#[derive(Debug, Clone)]
pub(crate) struct RenderedMarkup {
    pub markup: String,
    pub base_candidates: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
struct PreparedTemplate {
    main_name: String,
    main_source: Option<String>,
    base_candidates: Vec<PathBuf>,
}

pub(crate) fn render_markup(
    request: &RenderRequest,
    repository: TemplateRepository,
) -> Result<RenderedMarkup> {
    let prepared = prepare_template(request, &repository)?;
    let context = parse_context_json(&request.context_json)?;
    let environment = build_environment(&repository, &prepared)?;
    let template = environment.get_template(&prepared.main_name)?;
    let markup = template.render(context)?;

    Ok(RenderedMarkup {
        markup,
        base_candidates: prepared.base_candidates,
    })
}

fn prepare_template(request: &RenderRequest, repository: &TemplateRepository) -> Result<PreparedTemplate> {
    let template_name = request.template_name.as_deref().map(str::trim).filter(|value| !value.is_empty());
    let template_file = request.template_file.as_deref().map(str::trim).filter(|value| !value.is_empty());
    let template_source = request.template_source.as_deref().filter(|value| !value.trim().is_empty());

    match (template_source, template_file, template_name) {
        (Some(source), None, maybe_name) => Ok(PreparedTemplate {
            main_name: maybe_name.unwrap_or("inline.html").to_string(),
            main_source: Some(source.to_string()),
            base_candidates: repository.search_paths.clone(),
        }),
        (None, Some(file), None) => {
            let path = resolve_existing_template_path(file, &repository.search_paths)?
                .ok_or_else(|| RendererError::template_not_found(file))?;
            Ok(PreparedTemplate {
                main_name: path.to_string_lossy().into_owned(),
                main_source: None,
                base_candidates: collect_base_candidates(path.parent(), &repository.search_paths),
            })
        }
        (None, None, Some(name)) => {
            if repository.registered_templates.contains_key(name) {
                return Ok(PreparedTemplate {
                    main_name: name.to_string(),
                    main_source: None,
                    base_candidates: repository.search_paths.clone(),
                });
            }

            let path = resolve_existing_template_path(name, &repository.search_paths)?
                .ok_or_else(|| RendererError::template_not_found(name))?;
            Ok(PreparedTemplate {
                main_name: path.to_string_lossy().into_owned(),
                main_source: None,
                base_candidates: collect_base_candidates(path.parent(), &repository.search_paths),
            })
        }
        (None, None, None) => Err(RendererError::invalid_request(
            "exactly one of template_source, template_file, or template_name must be provided",
        )),
        _ => Err(RendererError::invalid_request(
            "template_source, template_file, and template_name are mutually exclusive except that template_name may accompany template_source as its logical name",
        )),
    }
}

fn build_environment(
    repository: &TemplateRepository,
    prepared: &PreparedTemplate,
) -> Result<Environment<'static>> {
    let registered_templates = repository.registered_templates.clone();
    let registered_names = Arc::new(
        registered_templates
            .keys()
            .cloned()
            .collect::<HashSet<String>>(),
    );
    let search_paths = repository.search_paths.clone();
    let file_cache = Arc::clone(&repository.file_cache);

    let mut env = Environment::new();
    env.set_auto_escape_callback(|name| {
        if should_auto_escape(name) {
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

    if let Some(source) = prepared.main_source.as_ref() {
        env.add_template_owned(prepared.main_name.clone(), source.clone())?;
    }

    env.add_filter("markdown", markdown_filter);
    env.add_filter("datetime_format", datetime_format_filter);
    env.add_filter("filesize", filesize_filter);
    env.add_filter("to_hex", to_hex_filter);
    env.add_filter("json_pretty", json_pretty_filter);
    env.add_filter("highlight", highlight_filter);

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

fn push_unique_path(paths: &mut Vec<PathBuf>, candidate: PathBuf) {
    if !paths.iter().any(|existing| existing == &candidate) {
        paths.push(candidate);
    }
}

fn should_auto_escape(name: &str) -> bool {
    [".html", ".htm", ".xhtml", ".xml", ".jinja", ".jinja2", ".j2"]
        .iter()
        .any(|suffix| name.ends_with(suffix))
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


/// Markdown 转 HTML 过滤器
/// 用法: {{ content | markdown }}
fn markdown_filter(v: String) -> JinjaValue {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    
    let parser = Parser::new_ext(&v, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    
    JinjaValue::from_safe_string(html_output)
}

/// 毫秒级时间戳转可读时间字符串
/// 用法: {{ ts | datetime_format("%Y-%m-%d %H:%M:%S") }}
fn datetime_format_filter(ts: i64, format_str: &str) -> std::result::Result<String, Error> {
    use chrono::{DateTime, TimeZone, Utc};

    let dt = if ts.abs() > 9_999_999_999 {
        DateTime::from_timestamp_millis(ts)
    } else {
        Utc.timestamp_opt(ts, 0).single()
    };

    let dt = dt.ok_or_else(|| {
        Error::new(ErrorKind::InvalidOperation, "非法的时间戳数据")
    })?;

    Ok(dt.format(format_str).to_string())
}

/// 字节大小自动转换 (B, KB, MB, GB)
/// 用法: {{ bytes | filesize }}
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

/// 数字转十六进制
/// 用法: {{ reg_addr | to_hex }} -> 0x00FF
/// 用法: {{ reg_addr | to_hex(width=2) }} -> 0xFF
fn to_hex_filter(val: u64, kwargs: minijinja::value::Kwargs) -> std::result::Result<String, Error> {
    let width: usize = kwargs.get("width").unwrap_or(4);
    kwargs.assert_all_used()?; // 确保没有传错其他参数
    
    Ok(format!("0x{:0>width$X}", val, width = width))
}

/// JSON 漂亮打印
/// 用法: <pre><code>{{ config_obj | json_pretty }}</code></pre>
fn json_pretty_filter(val: JinjaValue) -> std::result::Result<JinjaValue, Error> {
    let serialized = serde_json::to_string_pretty(&val)
        .map_err(|e| Error::new(ErrorKind::InvalidOperation, e.to_string()))?;
    Ok(JinjaValue::from_safe_string(serialized))
}

use std::sync::LazyLock;

use syntect::{
    easy::HighlightLines,
    highlighting::{Theme, ThemeSet},
    html::{
        styled_line_to_highlighted_html,
        IncludeBackground,
    },
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

static SYNTAX_SET: LazyLock<SyntaxSet> =
    LazyLock::new(SyntaxSet::load_defaults_newlines);

static THEME: LazyLock<Theme> = LazyLock::new(|| {
    let ts = ThemeSet::load_defaults();
    ts.themes["base16-ocean.dark"].clone()
});

fn highlight_filter(code: &str, lang: &str) -> JinjaValue {
    let syntax = SYNTAX_SET
        .find_syntax_by_token(lang)
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

    let mut highlighter =
        HighlightLines::new(syntax, &THEME);

    let mut html = String::new();

    html.push_str("<pre><code>");

    for line in LinesWithEndings::from(code) {
        let ranges = highlighter
            .highlight_line(line, &SYNTAX_SET)
            .unwrap();

        let line_html =
            styled_line_to_highlighted_html(
                &ranges,
                IncludeBackground::No,
            )
            .unwrap();

        html.push_str(&line_html);
    }

    html.push_str("</code></pre>");

    JinjaValue::from_safe_string(html)
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::{Arc, Mutex}};

    use tempfile::TempDir;

    use super::*;
    use crate::{api::{ImageFormat, RenderRequest, RenderSize}, cache::FileCache};

    fn render_inline(template_source: &str, context_json: &str) -> String {
        let request = RenderRequest {
            template_name: Some("inline.jinja".to_string()),
            template_file: None,
            template_source: Some(template_source.to_string()),
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

        render_markup(&request, repository)
            .expect("render inline template")
            .markup
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
            template_name: None,
            template_file: Some(template_dir.join("index.jinja").to_string_lossy().into_owned()),
            template_source: None,
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

        let rendered = render_markup(&request, repository).expect("render file template");
        assert_eq!(rendered.markup, "<div>Footer</div>");
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
}
