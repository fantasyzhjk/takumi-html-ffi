use std::{
    collections::{BTreeMap, HashMap},
    sync::LazyLock,
};

use takumi::prelude::{Direction, Lang, Node, Style, StyleDeclarationBlock};

use super::{Result, parser::ParsedElement, resource::AssetResolver};

const DEFAULT_PRESETS: &[(&str, &str)] = &[
    ("html", "display:block"),
    ("head", "display:none"),
    ("meta", "display:none"),
    ("title", "display:none"),
    ("link", "display:none"),
    ("style", "display:none"),
    ("script", "display:none"),
    ("noscript", "display:none"),
    ("datalist", "display:none"),
    ("template", "display:none"),
    ("body", "margin:8px;display:block"),
    ("p", "margin-top:1em;margin-bottom:1em;display:block"),
    (
        "blockquote",
        "margin-top:1em;margin-bottom:1em;margin-left:40px;margin-right:40px;display:block",
    ),
    (
        "figure",
        "margin-top:1em;margin-bottom:1em;margin-left:40px;margin-right:40px;display:block",
    ),
    ("figcaption", "display:block"),
    ("address", "font-style:italic;display:block"),
    ("article", "display:block"),
    ("aside", "display:block"),
    ("footer", "display:block"),
    ("header", "display:block"),
    ("hgroup", "display:block"),
    ("main", "display:block"),
    ("nav", "display:block"),
    ("section", "display:block"),
    ("center", "text-align:center;display:block"),
    (
        "hr",
        "margin-top:0.5em;margin-bottom:0.5em;margin-left:auto;margin-right:auto;border-width:1px;display:block",
    ),
    (
        "ul",
        "margin-top:1em;margin-bottom:1em;padding-inline-start:40px;display:block;list-style-type:disc",
    ),
    (
        "ol",
        "margin-top:1em;margin-bottom:1em;padding-inline-start:40px;display:block;list-style-type:decimal",
    ),
    (
        "menu",
        "margin-top:1em;margin-bottom:1em;padding-inline-start:40px;display:block;list-style-type:disc",
    ),
    ("li", "display:list-item"),
    ("dl", "margin-top:1em;margin-bottom:1em;display:block"),
    ("dt", "display:block"),
    ("dd", "margin-left:40px;display:block"),
    ("form", "display:block"),
    (
        "fieldset",
        "margin-left:2px;margin-right:2px;padding-top:0.35em;padding-right:0.75em;padding-bottom:0.625em;padding-left:0.75em;border-width:2px;display:block",
    ),
    ("legend", "padding-left:2px;padding-right:2px;display:block"),
    ("details", "display:block"),
    ("summary", "display:block"),
    ("search", "display:block"),
    (
        "h1",
        "font-size:2em;margin-top:0.67em;margin-bottom:0.67em;margin-left:0;margin-right:0;font-weight:bold;display:block",
    ),
    (
        "h2",
        "font-size:1.5em;margin-top:0.83em;margin-bottom:0.83em;margin-left:0;margin-right:0;font-weight:bold;display:block",
    ),
    (
        "h3",
        "font-size:1.17em;margin-top:1em;margin-bottom:1em;margin-left:0;margin-right:0;font-weight:bold;display:block",
    ),
    (
        "h4",
        "margin-top:1.33em;margin-bottom:1.33em;margin-left:0;margin-right:0;font-weight:bold;display:block",
    ),
    (
        "h5",
        "font-size:0.83em;margin-top:1.67em;margin-bottom:1.67em;margin-left:0;margin-right:0;font-weight:bold;display:block",
    ),
    (
        "h6",
        "font-size:0.67em;margin-top:2.33em;margin-bottom:2.33em;margin-left:0;margin-right:0;font-weight:bold;display:block",
    ),
    ("u", "text-decoration:underline"),
    ("ins", "text-decoration:underline"),
    ("strong", "font-weight:bolder"),
    ("b", "font-weight:bolder"),
    ("i", "font-style:italic"),
    ("em", "font-style:italic"),
    ("cite", "font-style:italic"),
    ("dfn", "font-style:italic"),
    ("code", "font-family:monospace"),
    ("kbd", "font-family:monospace"),
    ("samp", "font-family:monospace"),
    (
        "pre",
        "font-family:monospace;white-space:pre;margin:1em 0;display:block",
    ),
    ("mark", "background-color:yellow;color:black"),
    ("big", "font-size:larger"),
    ("small", "font-size:smaller"),
    ("s", "text-decoration:line-through"),
    ("del", "text-decoration:line-through"),
    ("sub", "font-size:smaller;vertical-align:sub"),
    ("sup", "font-size:smaller;vertical-align:super"),
    ("div", "display:block"),
    ("br", "white-space:pre"),
    (
        "table",
        "display:table;box-sizing:border-box;border-spacing:2px",
    ),
    ("thead", "display:table-header-group"),
    ("tbody", "display:table-row-group"),
    ("tfoot", "display:table-footer-group"),
    ("tr", "display:table-row"),
    ("td", "display:table-cell;padding:1px"),
    (
        "th",
        "display:table-cell;padding:1px;font-weight:bold;text-align:center",
    ),
    ("caption", "display:table-caption;text-align:center"),
];

static DEFAULT_STYLE_PRESETS: LazyLock<HashMap<Box<str>, Style>> = LazyLock::new(|| {
    DEFAULT_PRESETS
        .iter()
        .map(|&(tag, css)| {
            (
                tag.into(),
                Style::from(StyleDeclarationBlock::parse_loosy(css)),
            )
        })
        .collect()
});

fn default_preset(tag_name: &str) -> Option<Style> {
    DEFAULT_STYLE_PRESETS.get(tag_name).cloned()
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ExtractedMetadata {
    pub(crate) tag_name: Option<String>,
    pub(crate) class_name: Option<String>,
    pub(crate) id: Option<String>,
    pub(crate) dir: Option<Direction>,
    pub(crate) lang: Option<Lang>,
    pub(crate) attributes: BTreeMap<Box<str>, Box<str>>,
    pub(crate) inline_style: Option<Style>,
}

impl ExtractedMetadata {
    pub(crate) fn from_element(
        element: &ParsedElement,
        asset_resolver: &AssetResolver<'_>,
    ) -> Result<Self> {
        let class_name = element.attributes.get("class").cloned();
        let id = element.attributes.get("id").cloned();
        let dir = element.attributes.get("dir").and_then(|dir| parse_dir(dir));
        let lang = element
            .attributes
            .get("lang")
            .and_then(|lang| Lang::parse(lang).ok());
        let inline_style_text = element
            .attributes
            .get("style")
            .map(|style| asset_resolver.resolve_inline_style_attribute(style))
            .transpose()?;

        let mut attributes = BTreeMap::new();
        for (name, value) in &element.attributes {
            if is_reserved_attribute(name) {
                continue;
            }

            attributes.insert(name.as_str().into(), value.as_str().into());
        }

        Ok(Self {
            tag_name: Some(element.tag_name.clone()),
            class_name,
            id,
            dir,
            lang,
            attributes,
            inline_style: parse_inline_style(inline_style_text.as_deref()),
        })
    }

    pub(crate) fn apply_to(self, mut node: Node) -> Node {
        if let Some(tag_name) = self.tag_name {
            if let Some(preset) = default_preset(&tag_name) {
                node = node.with_preset(preset);
            }
            node = node.with_tag_name(tag_name);
        }

        if let Some(class_name) = self.class_name {
            node = node.with_class_name(class_name);
        }

        if let Some(id) = self.id {
            node = node.with_id(id);
        }

        if let Some(dir) = self.dir {
            node = node.with_dir(dir);
        }

        if let Some(lang) = self.lang {
            node = node.with_lang(lang);
        }

        if !self.attributes.is_empty() {
            node = node.with_attributes(self.attributes);
        }

        if let Some(style) = self.inline_style {
            node = node.with_style(style);
        }

        node
    }
}

fn parse_inline_style(style_text: Option<&str>) -> Option<Style> {
    let style_text = style_text.map(str::trim).filter(|style| !style.is_empty())?;

    Some(Style::from(StyleDeclarationBlock::parse_loosy(style_text)))
}

fn is_reserved_attribute(name: &str) -> bool {
    matches!(
        name,
        "class" | "dir" | "id" | "lang" | "src" | "style" | "tw"
    )
}

fn parse_dir(value: &str) -> Option<Direction> {
    if value.eq_ignore_ascii_case("ltr") {
        Some(Direction::Ltr)
    } else if value.eq_ignore_ascii_case("rtl") {
        Some(Direction::Rtl)
    } else {
        None
    }
}
