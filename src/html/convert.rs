use takumi::layout::node::Node;

use super::{
    FromHtmlOptions, FromHtmlResult, HtmlError, Result,
    emoji::split_text_to_nodes,
    metadata::ExtractedMetadata,
    parser::{ParsedElement, ParsedHtml, ParsedNode},
    resource::AssetResolver,
    style::{StyleAdapter, default_style_adapter},
};

#[derive(Debug, Clone, Copy, Default)]
struct TextContext {
    preserve_whitespace: bool,
}

pub(crate) fn convert(document: ParsedHtml, options: &FromHtmlOptions) -> Result<FromHtmlResult> {
    let style_adapter = default_style_adapter();
    let asset_resolver = AssetResolver::new(options);
    let mut stylesheets = Vec::new();
    let mut nodes = Vec::new();

    for node in document.nodes {
        if let Some(node) = convert_node(
            node,
            &style_adapter,
            &asset_resolver,
            options,
            &mut stylesheets,
            TextContext::default(),
        )? {
            nodes.push(node);
        }
    }

    Ok(FromHtmlResult {
        node: wrap_root_nodes(nodes),
        stylesheets,
    })
}

fn convert_node(
    node: ParsedNode,
    style_adapter: &impl StyleAdapter,
    asset_resolver: &AssetResolver<'_>,
    options: &FromHtmlOptions,
    stylesheets: &mut Vec<String>,
    text_context: TextContext,
) -> Result<Option<Node>> {
    match node {
        ParsedNode::Text(text) => {
            let Some(text) = normalize_text_node(&text, options, text_context) else {
                return Ok(None);
            };
            if text.is_empty() {
                Ok(None)
            } else {
                Ok(Some(build_text_node(text, options)?))
            }
        }
        ParsedNode::Element(element) => convert_element(
            element,
            style_adapter,
            asset_resolver,
            options,
            stylesheets,
            text_context,
        ),
    }
}

fn convert_element(
    element: ParsedElement,
    style_adapter: &impl StyleAdapter,
    asset_resolver: &AssetResolver<'_>,
    options: &FromHtmlOptions,
    stylesheets: &mut Vec<String>,
    text_context: TextContext,
) -> Result<Option<Node>> {
    if element.tag_name == "link" {
        if !is_stylesheet_link(&element) {
            return Ok(None);
        }

        let Some(href) = element.attributes.get("href").map(String::as_str) else {
            return Err(HtmlError::MissingLinkedStylesheetHref {
                tag_name: element.tag_name,
            });
        };

        if let Some(stylesheet) = asset_resolver.load_linked_stylesheet(href)? {
            stylesheets.push(stylesheet);
        }
        return Ok(None);
    }

    if element.tag_name == "style" {
        let stylesheet = collect_direct_text(&element.children);
        let stylesheet = asset_resolver.resolve_inline_stylesheet(&stylesheet)?;
        if !stylesheet.is_empty() {
            stylesheets.push(stylesheet);
        }
        return Ok(None);
    }

    if element.tag_name == "head" {
        let _ = convert_children(
            element.children,
            style_adapter,
            asset_resolver,
            options,
            stylesheets,
            text_context,
        )?;
        return Ok(None);
    }

    if is_non_rendered_element(&element.tag_name) {
        return Ok(None);
    }

    let metadata = ExtractedMetadata::from_element(&element, style_adapter, asset_resolver)?;

    if element.tag_name == "br" {
        return Ok(Some(metadata.apply_to(Node::text("\n"))));
    }

    if element.tag_name == "img" {
        let Some(src) = element.attributes.get("src") else {
            return Err(HtmlError::MissingImageSource {
                tag_name: element.tag_name,
            });
        };
        let src = asset_resolver.resolve_image_source(src)?;

        let width = parse_dimension(element.attributes.get("width").map(String::as_str));
        let height = parse_dimension(element.attributes.get("height").map(String::as_str));

        return Ok(Some(metadata.apply_to(Node::image((
            src.as_str(),
            width,
            height,
        )))));
    }

    if element.tag_name == "svg" {
        let svg_markup = normalize_inline_svg_markup(
            element.serialized_html.as_deref().unwrap_or_default(),
            &element,
        );
        let width = parse_dimension(element.attributes.get("width").map(String::as_str));
        let height = parse_dimension(element.attributes.get("height").map(String::as_str));

        return Ok(Some(
            metadata.apply_to(Node::image((svg_markup, width, height))),
        ));
    }

    if is_void_element(&element.tag_name) {
        return Ok(None);
    }

    let child_text_context = text_context.with_tag(&element.tag_name);

    if let Some(text_content) = direct_text_content(&element.children, options, child_text_context)
    {
        return Ok(Some(build_element_text_node(
            metadata,
            text_content,
            options,
        )?));
    }

    let children = convert_children(
        element.children,
        style_adapter,
        asset_resolver,
        options,
        stylesheets,
        child_text_context,
    )?;
    Ok(Some(metadata.apply_to(Node::container(children))))
}

fn convert_children(
    children: Vec<ParsedNode>,
    style_adapter: &impl StyleAdapter,
    asset_resolver: &AssetResolver<'_>,
    options: &FromHtmlOptions,
    stylesheets: &mut Vec<String>,
    text_context: TextContext,
) -> Result<Vec<Node>> {
    let mut nodes = Vec::new();

    for child in children {
        if let Some(node) = convert_node(
            child,
            style_adapter,
            asset_resolver,
            options,
            stylesheets,
            text_context,
        )? {
            nodes.push(node);
        }
    }

    Ok(nodes)
}

fn collect_direct_text(children: &[ParsedNode]) -> String {
    let mut text = String::new();

    for child in children {
        if let ParsedNode::Text(value) = child {
            text.push_str(value);
        }
    }

    text
}

fn direct_text_content(
    children: &[ParsedNode],
    options: &FromHtmlOptions,
    text_context: TextContext,
) -> Option<String> {
    if children.is_empty() {
        return None;
    }

    let mut text = String::new();

    for child in children {
        match child {
            ParsedNode::Text(value) => text.push_str(value),
            ParsedNode::Element(_) => return None,
        }
    }

    normalize_text_run(&text, options, text_context, true)
}

fn parse_dimension(value: Option<&str>) -> Option<f32> {
    let value = value?.trim();
    if value.is_empty() {
        return None;
    }

    value
        .parse::<f32>()
        .ok()
        .or_else(|| value.strip_suffix("px")?.trim().parse::<f32>().ok())
}

fn build_text_node(text: String, options: &FromHtmlOptions) -> Result<Node> {
    if let Some(emoji_type) = options.emoji_type
        && let Some(nodes) = split_text_to_nodes(&text, emoji_type)?
    {
        return Ok(Node::container(nodes).with_tag_name("span"));
    }

    Ok(Node::text(text))
}

fn build_element_text_node(
    metadata: ExtractedMetadata,
    text: String,
    options: &FromHtmlOptions,
) -> Result<Node> {
    if let Some(emoji_type) = options.emoji_type
        && let Some(nodes) = split_text_to_nodes(&text, emoji_type)?
    {
        return Ok(metadata.apply_to(Node::container(nodes)));
    }

    Ok(metadata.apply_to(Node::text(text)))
}

fn normalize_inline_svg_markup(svg_markup: &str, element: &ParsedElement) -> String {
    let needs_xmlns = !svg_markup.contains("xmlns=");
    let needs_xlink_xmlns = (svg_markup.contains("xlink:")
        || element
            .attributes
            .keys()
            .any(|name| name.starts_with("xlink:")))
        && !svg_markup.contains("xmlns:xlink=");

    if !needs_xmlns && !needs_xlink_xmlns {
        return svg_markup.to_owned();
    }

    let Some(insert_at) = svg_markup.find("<svg").map(|index| index + 4) else {
        return svg_markup.to_owned();
    };

    let mut normalized = svg_markup.to_owned();
    let mut injected = String::new();
    if needs_xmlns {
        injected.push_str(" xmlns=\"http://www.w3.org/2000/svg\"");
    }
    if needs_xlink_xmlns {
        injected.push_str(" xmlns:xlink=\"http://www.w3.org/1999/xlink\"");
    }
    normalized.insert_str(insert_at, &injected);
    normalized
}

fn wrap_root_nodes(mut nodes: Vec<Node>) -> Node {
    match nodes.len() {
        0 => Node::container([]),
        1 => nodes.pop().unwrap_or_else(|| Node::container([])),
        _ => Node::container(nodes),
    }
}

fn is_void_element(tag_name: &str) -> bool {
    matches!(
        tag_name,
        "area"
            | "base"
            | "col"
            | "embed"
            | "hr"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

fn is_non_rendered_element(tag_name: &str) -> bool {
    matches!(tag_name, "noscript" | "script" | "template" | "title")
}

fn is_stylesheet_link(element: &ParsedElement) -> bool {
    element
        .attributes
        .get("rel")
        .map(|rel| {
            rel.split_ascii_whitespace()
                .any(|value| value.eq_ignore_ascii_case("stylesheet"))
        })
        .unwrap_or(false)
}

fn normalize_text_node(
    text: &str,
    options: &FromHtmlOptions,
    text_context: TextContext,
) -> Option<String> {
    normalize_text_run(text, options, text_context, false)
}

fn normalize_text_run(
    text: &str,
    options: &FromHtmlOptions,
    text_context: TextContext,
    trim_edges: bool,
) -> Option<String> {
    if text.is_empty() {
        return None;
    }

    if !options.normalize_whitespace || text_context.preserve_whitespace {
        return Some(text.to_owned());
    }

    if text.chars().all(is_collapsible_whitespace) {
        return None;
    }

    let mut normalized = String::with_capacity(text.len());
    let mut last_was_whitespace = false;

    for ch in text.chars() {
        if is_collapsible_whitespace(ch) {
            if !last_was_whitespace {
                normalized.push(' ');
                last_was_whitespace = true;
            }
        } else {
            normalized.push(ch);
            last_was_whitespace = false;
        }
    }

    if trim_edges {
        normalized = normalized.trim().to_owned();
    }

    (!normalized.is_empty()).then_some(normalized)
}

fn is_collapsible_whitespace(ch: char) -> bool {
    matches!(ch, '\u{0009}' | '\u{000A}' | '\u{000C}' | '\u{000D}' | ' ')
}

impl TextContext {
    fn with_tag(self, tag_name: &str) -> Self {
        Self {
            preserve_whitespace: self.preserve_whitespace || preserves_whitespace(tag_name),
        }
    }
}

fn preserves_whitespace(tag_name: &str) -> bool {
    matches!(tag_name, "pre" | "textarea" | "script" | "style")
}
