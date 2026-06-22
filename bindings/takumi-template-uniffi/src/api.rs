#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct InlineTemplateInput {
    pub source: String,
    pub logical_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum TemplateInput {
    Inline(InlineTemplateInput),
    File(String),
    Registered(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum TemplateContentKind {
    Markdown,
    JinjaHtml,
    JinjaMarkdown,
}

impl TemplateContentKind {
    pub(crate) fn requires_jinja(self) -> bool {
        matches!(self, Self::JinjaHtml | Self::JinjaMarkdown)
    }

    pub(crate) fn requires_markdown(self) -> bool {
        matches!(self, Self::Markdown | Self::JinjaMarkdown)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TemplateRequest {
    pub input: TemplateInput,
    pub context_json: Option<String>,
    pub content_kind: TemplateContentKind,
    pub syntax_theme: Option<String>,
}
