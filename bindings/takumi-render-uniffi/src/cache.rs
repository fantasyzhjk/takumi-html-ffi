use std::{
    collections::{HashMap, HashSet},
    fs,
    hash::{Hash, Hasher},
    io,
    path::{Path, PathBuf},
    sync::Arc,
    time::UNIX_EPOCH,
};

use takumi::prelude::ImageSource;

pub(crate) const DEFAULT_IMAGE_CACHE_MAX_ENTRIES: usize = 256;
pub(crate) const DEFAULT_IMAGE_CACHE_INACTIVE_PASSES: u64 = 64;

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
}

#[derive(Debug, Clone)]
struct CachedImage {
    signature: FileSignature,
    image: ImageSource,
    last_used_pass: u64,
}

#[derive(Debug)]
pub(crate) struct ImageCache {
    entries: HashMap<String, CachedImage>,
    enabled: bool,
    next_pass: u64,
    max_entries: usize,
    inactive_passes: u64,
}

impl Default for ImageCache {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            enabled: true,
            next_pass: 0,
            max_entries: DEFAULT_IMAGE_CACHE_MAX_ENTRIES,
            inactive_passes: DEFAULT_IMAGE_CACHE_INACTIVE_PASSES,
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

    pub(crate) fn next_pass(&mut self) -> u64 {
        self.next_pass = self.next_pass.saturating_add(1);
        self.next_pass
    }

    pub(crate) fn get_auto(
        &mut self,
        key: &str,
        signature: &FileSignature,
        pass: u64,
    ) -> Option<ImageSource> {
        let cached = self.entries.get_mut(key)?;
        if &cached.signature != signature {
            return None;
        }
        cached.last_used_pass = pass;
        Some(cached.image.clone())
    }

    pub(crate) fn insert(
        &mut self,
        key: String,
        signature: FileSignature,
        image: ImageSource,
        pass: u64,
    ) {
        self.entries.insert(
            key,
            CachedImage {
                signature,
                image,
                last_used_pass: pass,
            },
        );
    }

    pub(crate) fn prune(&mut self, pass: u64) -> bool {
        let before = self.entries.len();
        let inactive_passes = self.inactive_passes;
        self.entries
            .retain(|_, image| pass.saturating_sub(image.last_used_pass) <= inactive_passes);

        if self.entries.len() > self.max_entries {
            let remove_count = self.entries.len() - self.max_entries;
            let mut keys_by_age = self
                .entries
                .iter()
                .map(|(key, image)| (key.clone(), image.last_used_pass))
                .collect::<Vec<_>>();
            keys_by_age.sort_by_key(|(_, last_used_pass)| *last_used_pass);
            for (key, _) in keys_by_age.into_iter().take(remove_count) {
                self.entries.remove(&key);
            }
        }

        before != self.entries.len()
    }

    pub(crate) fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

#[derive(Debug, Clone)]
struct CachedFile {
    signature: FileSignature,
    bytes: Arc<Vec<u8>>,
}

#[derive(Debug, Default)]
pub(crate) struct FileCache {
    entries: HashMap<PathBuf, CachedFile>,
}

impl FileCache {
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }

    pub(crate) fn read_bytes(&mut self, path: &Path) -> io::Result<Arc<Vec<u8>>> {
        let normalized = normalize_existing_path(path)?;
        let signature = FileSignature::from_path(&normalized)?;

        if let Some(cached) = self.entries.get(&normalized)
            && cached.signature == signature
        {
            return Ok(Arc::clone(&cached.bytes));
        }

        let bytes = Arc::new(fs::read(&normalized)?);
        self.entries.insert(
            normalized,
            CachedFile {
                signature,
                bytes: Arc::clone(&bytes),
            },
        );
        Ok(bytes)
    }

    pub(crate) fn read_string(&mut self, path: &Path) -> crate::Result<String> {
        let bytes = self.read_bytes(path)?;
        Ok(String::from_utf8(bytes.as_ref().clone())?)
    }
}

#[derive(Debug, Default)]
pub(crate) struct FontCache {
    loaded_paths: HashSet<PathBuf>,
    loaded_hashes: HashSet<u64>,
}

impl FontCache {
    pub(crate) fn clear(&mut self) {
        self.loaded_paths.clear();
        self.loaded_hashes.clear();
    }

    pub(crate) fn contains_path(&self, path: &Path) -> bool {
        self.loaded_paths.contains(path)
    }

    pub(crate) fn contains_hash(&self, hash: u64) -> bool {
        self.loaded_hashes.contains(&hash)
    }

    pub(crate) fn remember_path(&mut self, path: PathBuf) {
        self.loaded_paths.insert(path);
    }

    pub(crate) fn remember_hash(&mut self, hash: u64) {
        self.loaded_hashes.insert(hash);
    }

    pub(crate) fn entry_count(&self) -> usize {
        self.loaded_hashes.len()
    }
}

pub(crate) fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
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
