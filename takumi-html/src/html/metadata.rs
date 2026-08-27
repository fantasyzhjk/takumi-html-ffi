use std::{collections::BTreeMap, str::FromStr};

use takumi::prelude::{Direction, Node, Style, StyleDeclarationBlock};

use super::{HtmlError, Result, parser::ParsedElement, resource::AssetResolver};

#[derive(Debug, Clone, Default)]
pub(crate) struct ExtractedMetadata {
    pub(crate) tag_name: Option<String>,
    pub(crate) class_name: Option<String>,
    pub(crate) id: Option<String>,
    pub(crate) dir: Option<Direction>,
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
            attributes,
            inline_style: parse_inline_style(&element.tag_name, inline_style_text.as_deref())?,
        })
    }

    pub(crate) fn apply_to(self, mut node: Node) -> Node {
        if let Some(tag_name) = self.tag_name {
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

        if !self.attributes.is_empty() {
            node = node.with_attributes(self.attributes);
        }

        if let Some(style) = self.inline_style {
            node = node.with_style(style);
        }

        node
    }
}

fn parse_inline_style(tag_name: &str, style_text: Option<&str>) -> Result<Option<Style>> {
    let Some(style_text) = style_text.map(str::trim).filter(|style| !style.is_empty()) else {
        return Ok(None);
    };

    let declarations = StyleDeclarationBlock::from_str(style_text).map_err(|error| {
        HtmlError::InlineStyleParse {
            tag_name: tag_name.to_owned(),
            value: style_text.to_owned(),
            reason: error.to_string(),
        }
    })?;

    Ok(Some(declarations.into()))
}

fn is_reserved_attribute(name: &str) -> bool {
    matches!(name, "class" | "dir" | "id" | "src" | "style" | "tw")
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
