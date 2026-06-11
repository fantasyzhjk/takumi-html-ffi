use std::str::FromStr;

use takumi::layout::style::{Style, StyleDeclarationBlock};

use super::{HtmlError, Result};

pub(crate) trait StyleAdapter {
    fn inline_style(&self, tag_name: &str, style_text: Option<&str>) -> Result<Option<Style>>;
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct PlaceholderStyleAdapter;

pub(crate) fn default_style_adapter() -> PlaceholderStyleAdapter {
    PlaceholderStyleAdapter
}

impl StyleAdapter for PlaceholderStyleAdapter {
    fn inline_style(&self, tag_name: &str, style_text: Option<&str>) -> Result<Option<Style>> {
        parse_inline_style(tag_name, style_text)
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
