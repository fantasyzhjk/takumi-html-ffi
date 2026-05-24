use std::path::PathBuf;

use super::emoji::EmojiType;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LocalAssetMode {
    #[default]
    Preserve,
    InlineDataUri,
    AbsolutePath,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FromHtmlOptions {
    pub base_path: Option<PathBuf>,
    pub load_linked_stylesheets: bool,
    pub resolve_local_assets: bool,
    pub local_asset_mode: Option<LocalAssetMode>,
    pub normalize_whitespace: bool,
    pub emoji_type: Option<EmojiType>,
}

impl FromHtmlOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_base_path<P>(mut self, base_path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        self.base_path = Some(base_path.into());
        self
    }

    pub fn load_linked_stylesheets(mut self, enabled: bool) -> Self {
        self.load_linked_stylesheets = enabled;
        self
    }

    pub fn resolve_local_assets(mut self, enabled: bool) -> Self {
        self.resolve_local_assets = enabled;
        self.local_asset_mode = None;
        self
    }

    pub fn local_asset_mode(mut self, mode: LocalAssetMode) -> Self {
        self.resolve_local_assets = matches!(mode, LocalAssetMode::InlineDataUri);
        self.local_asset_mode = Some(mode);
        self
    }

    pub fn normalize_whitespace(mut self, enabled: bool) -> Self {
        self.normalize_whitespace = enabled;
        self
    }

    pub fn emoji_type(mut self, emoji_type: EmojiType) -> Self {
        self.emoji_type = Some(emoji_type);
        self
    }

    pub(crate) fn effective_local_asset_mode(&self) -> LocalAssetMode {
        self.local_asset_mode.unwrap_or(if self.resolve_local_assets {
            LocalAssetMode::InlineDataUri
        } else {
            LocalAssetMode::Preserve
        })
    }
}
