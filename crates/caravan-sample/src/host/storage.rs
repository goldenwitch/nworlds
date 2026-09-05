pub use nworlds_host::StorageTransport;
pub type MemoryStorage = nworlds_host::MemoryStorage;

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
