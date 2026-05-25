use std::{
    collections::{HashMap, HashSet},
    fs,
    hash::{Hash, Hasher},
    io,
    path::{Path, PathBuf},
    sync::Arc,
    time::UNIX_EPOCH,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileSignature {
    len: u64,
    modified_nanos: Option<u128>,
}

impl FileSignature {
    fn from_path(path: &Path) -> io::Result<Self> {
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
