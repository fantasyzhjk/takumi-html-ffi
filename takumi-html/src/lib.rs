pub mod html;

pub use crate::html::{
    EmojiType, FromHtmlOptions, FromHtmlResult, HtmlError, LocalAssetMode,
    Result as HtmlResult, from_document, from_document_with_options, from_fragment,
    from_fragment_with_options, from_html,
    from_html_with_options,
};
