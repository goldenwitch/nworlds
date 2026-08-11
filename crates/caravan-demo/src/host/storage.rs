/// A target-facing transport port for already encoded persistence bytes.
pub trait StorageTransport {
    /// Replaces the stored encoded record.
    fn store(&mut self, bytes: Vec<u8>);

    /// Returns an owned copy of the stored encoded record, if present.
    fn load(&self) -> Option<Vec<u8>>;
}

/// In-memory byte storage for tests and the first target composition.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MemoryStorage {
    bytes: Option<Vec<u8>>,
}

impl MemoryStorage {
    /// Creates empty in-memory storage.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reports whether one encoded record is stored.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_none()
    }
}

impl StorageTransport for MemoryStorage {
    fn store(&mut self, bytes: Vec<u8>) {
        self.bytes = Some(bytes);
    }

    fn load(&self) -> Option<Vec<u8>> {
        self.bytes.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::{MemoryStorage, StorageTransport};

    #[test]
    fn memory_storage_retains_and_returns_owned_bytes() {
        let mut storage = MemoryStorage::new();
        assert!(storage.is_empty());

        storage.store(vec![1, 2, 3]);
        let loaded = storage.load().expect("stored bytes should load");

        assert_eq!(loaded, vec![1, 2, 3]);
        assert_eq!(storage.load(), Some(vec![1, 2, 3]));
        assert!(!storage.is_empty());
    }
}
