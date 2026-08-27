use std::{
    collections::HashMap, fs, io, path::{Path, PathBuf}, time::UNIX_EPOCH
};

use takumi::prelude::ImageSource;

pub(crate) const DEFAULT_IMAGE_CACHE_MAX_ENTRIES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileSignature {
    len: u64,
    modified_nanos: Option<u128>,
}

impl FileSignature {
    pub(crate) fn from_path(path: &Path) -> io::Result<Self> {
        let metadata = fs::metadata(path)?;
        let modified_nanos = metadata
            .modified()
            .ok()
            .and_then(|timestamp| timestamp.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_nanos());

        Ok(Self {
            len: metadata.len(),
            modified_nanos,
        })
    }

    /// Placeholder signature for remote URLs. Matches itself so cached entries
    /// stay valid until LRU eviction.
    #[cfg(feature = "http")]
    pub(crate) fn remote() -> Self {
        Self {
            len: 0,
            modified_nanos: None,
        }
    }
}

#[derive(Debug, Clone)]
struct CachedImage {
    signature: FileSignature,
    image: ImageSource,
    access_seq: u64,
}

#[derive(Debug)]
pub(crate) struct ImageCache {
    entries: HashMap<String, CachedImage>,
    enabled: bool,
    access_seq: u64,
    max_entries: usize,
}

impl Default for ImageCache {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            enabled: true,
            access_seq: 0,
            max_entries: DEFAULT_IMAGE_CACHE_MAX_ENTRIES,
        }
    }
}

impl ImageCache {
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }

    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub(crate) fn set_enabled(&mut self, enabled: bool) {
        if self.enabled != enabled {
            self.entries.clear();
        }
        self.enabled = enabled;
    }

    pub(crate) fn get(
        &mut self,
        key: &str,
        signature: &FileSignature,
    ) -> Option<ImageSource> {
        let cached = self.entries.get_mut(key)?;
        if &cached.signature != signature {
            return None;
        }
        self.access_seq = self.access_seq.saturating_add(1);
        cached.access_seq = self.access_seq;
        Some(cached.image.clone())
    }

    pub(crate) fn insert(
        &mut self,
        key: String,
        signature: FileSignature,
        image: ImageSource,
    ) {
        if self.entries.len() >= self.max_entries && !self.entries.contains_key(&key) {
            if let Some(oldest_key) = self
                .entries
                .iter()
                .min_by_key(|(_, v)| v.access_seq)
                .map(|(k, _)| k.clone())
            {
                self.entries.remove(&oldest_key);
            }
        }
        self.access_seq = self.access_seq.saturating_add(1);
        self.entries.insert(
            key,
            CachedImage {
                signature,
                image,
                access_seq: self.access_seq,
            },
        );
    }

    pub(crate) fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

pub(crate) fn normalize_existing_path(path: &Path) -> io::Result<PathBuf> {
    if path.exists() {
        path.canonicalize()
    } else {
        absolute_path(path)
    }
}

pub(crate) fn absolute_path(path: &Path) -> io::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}
