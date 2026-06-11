use std::collections::BTreeMap;

use takumi::layout::{
    node::Node,
    style::{Direction, Style},
};

use super::{Result, parser::ParsedElement, resource::AssetResolver, style::StyleAdapter};

#[derive(Debug, Clone, Default)]
pub(crate) struct ExtractedMetadata {
    pub(crate) tag_name: Option<String>,
    pub(crate) class_name: Option<String>,
    pub(crate) id: Option<String>,
    pub(crate) dir: Option<Direction>,
    pub(crate) attributes: BTreeMap<Box<str>, Box<str>>,
    pub(crate) inline_style: Option<Style>,
    tailwind_input: Option<String>,
}

impl ExtractedMetadata {
    pub(crate) fn from_element(
        element: &ParsedElement,
        style_adapter: &impl StyleAdapter,
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
            inline_style: style_adapter.inline_style(&element.tag_name, inline_style_text.as_deref())?,
            tailwind_input: element.attributes.get("tw").cloned(),
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

        if let Some(_tailwind_input) = self.tailwind_input.as_deref() {
            // TODO: Plumb raw `tw` input through a future style adapter once we
            // decide how the HTML layer should materialize TailwindValues.
        }

        node
    }
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
