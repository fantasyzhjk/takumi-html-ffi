use std::collections::BTreeMap;

use ego_tree::NodeRef;
use scraper::{ElementRef, Html, node::Node as ScraperNode};

#[derive(Debug, Clone, Default)]
pub(crate) struct ParsedHtml {
    pub(crate) nodes: Vec<ParsedNode>,
}

#[derive(Debug, Clone)]
pub(crate) enum ParsedNode {
    Text(String),
    Element(ParsedElement),
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ParsedElement {
    pub(crate) tag_name: String,
    pub(crate) attributes: BTreeMap<String, String>,
    pub(crate) children: Vec<ParsedNode>,
    pub(crate) serialized_html: Option<String>,
}

pub(crate) fn parse_fragment(markup: &str) -> ParsedHtml {
    let document = Html::parse_fragment(markup);
    let mut nodes = Vec::new();

    for child in document.root_element().children() {
        collect_node(child, &mut nodes);
    }

    ParsedHtml { nodes }
}

pub(crate) fn parse_document(markup: &str) -> ParsedHtml {
    let document = Html::parse_document(markup);

    ParsedHtml {
        nodes: vec![ParsedNode::Element(parse_element(document.root_element()))],
    }
}

fn collect_node(node: NodeRef<'_, ScraperNode>, nodes: &mut Vec<ParsedNode>) {
    match node.value() {
        ScraperNode::Document | ScraperNode::Fragment => {
            for child in node.children() {
                collect_node(child, nodes);
            }
        }
        ScraperNode::Doctype(_)
        | ScraperNode::Comment(_)
        | ScraperNode::ProcessingInstruction(_) => {}
        ScraperNode::Text(text) => nodes.push(ParsedNode::Text(text.text.to_string())),
        ScraperNode::Element(_) => {
            if let Some(element) = ElementRef::wrap(node) {
                nodes.push(ParsedNode::Element(parse_element(element)));
            }
        }
    }
}

fn parse_element(element: ElementRef<'_>) -> ParsedElement {
    let tag_name = element.value().name().to_owned();
    let attributes = element
        .value()
        .attrs()
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
        .collect();

    let mut children = Vec::new();
    for child in element.children() {
        collect_node(child, &mut children);
    }

    ParsedElement {
        serialized_html: (tag_name == "svg").then(|| element.html()),
        tag_name,
        attributes,
        children,
    }
}
