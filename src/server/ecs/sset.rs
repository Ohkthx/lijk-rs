#![allow(dead_code)]

#[derive(Debug)]
struct Entry<T> {
    key: usize,
    value: T,
}

/// A sparse set is a data structure that allows for efficient insertion, deletion, and lookup of
#[derive(Debug)]
pub struct SparseSet<T> {
    dense: Vec<Entry<T>>, // Dense set of values.
    sparse: Vec<usize>,   // Sparse set of indices.
}

impl<T> SparseSet<T> {
    /// Invalid key constant.
    const INVALID_KEY: usize = usize::MAX;

    /// Creates a new sparse set with the given capacity.
    pub fn new() -> Self {
        Self {
            dense: vec![],
            sparse: vec![],
        }
    }

    /// Ensures that `self.sparse` is at least as long as `key + 1`.
    fn ensure_capacity(&mut self, key: usize) {
        if key >= self.sparse.len() {
            // Grow the sparse vector so that `key` is within range.
            self.sparse.resize(key + 1, Self::INVALID_KEY);
        }
    }

    /// Obtains the dense index for the key provided.
    fn get_dense_idx(&self, key: usize) -> Option<usize> {
        let dense_idx = self.sparse[key];
        if dense_idx < self.dense.len() {
            Some(dense_idx)
        } else {
            None
        }
    }

    /// Checks if the sparse set contains the key.
    pub fn has_key(&self, key: usize) -> bool {
        self.get_dense_idx(key).is_some()
    }

    /// Gets the amount of elements in the sparse set.
    pub fn length(&self) -> usize {
        self.dense.len()
    }

    /// Obtains a reference for the value associated with the key.
    pub fn get(&self, key: usize) -> Option<&T> {
        if let Some(dense_idx) = self.get_dense_idx(key) {
            Some(&self.dense[dense_idx].value)
        } else {
            None
        }
    }

    /// Obtains a mutable reference for the value associated with the key.
    pub fn get_mut(&mut self, key: usize) -> Option<&mut T> {
        if let Some(dense_idx) = self.get_dense_idx(key) {
            Some(&mut self.dense[dense_idx].value)
        } else {
            None
        }
    }

    /// Inserts a new value at the specified key in the sparse set.
    /// If the key is already present, it just overwrites it; otherwise
    /// it pushes a new entry to the dense storage.
    pub fn insert(&mut self, key: usize, value: T) {
        // Ensure our sparse array is large enough to hold `key`.
        self.ensure_capacity(key);

        if let Some(stored) = self.get_mut(key) {
            // Key already present, just overwrite.
            *stored = value;
        } else {
            // Key not present; store a new entry.
            let dense_idx = self.dense.len();
            self.dense.push(Entry { key, value });
            self.sparse[key] = dense_idx;
        }
    }

    /// Removes a value based on the key provided.
    pub fn remove(&mut self, key: usize) -> Option<T> {
        if !self.has_key(key) {
            return None;
        }

        let dense_idx = self.sparse[key];
        let entry = self.dense.swap_remove(dense_idx);
        if (dense_idx) < self.dense.len() {
            let swapped = &self.dense[dense_idx];
            self.sparse[swapped.key] = dense_idx;
        }

        self.sparse[key] = Self::INVALID_KEY;
        Some(entry.value)
    }

    /// Allows for iteration over the dense set.
    pub fn iter(&self) -> impl Iterator<Item = (usize, &T)> {
        self.dense.iter().map(|entry| (entry.key, &entry.value))
    }

    /// Allows for iteration over the dense set.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (usize, &mut T)> {
        self.dense
            .iter_mut()
            .map(|entry| (entry.key, &mut entry.value))
    }

    /// Removes all values that match the predicate `f`.
    pub fn drain_if<F>(&mut self, mut f: F) -> impl Iterator<Item = (usize, T)>
    where
        F: FnMut(&T) -> bool,
    {
        let mut removed = vec![];
        let mut dense_idx = 0;

        while dense_idx < self.dense.len() {
            if f(&self.dense[dense_idx].value) {
                // Extract and save the key / value to be returned.
                let Entry { key, value } = self.dense.swap_remove(dense_idx);
                removed.push((key, value));

                // Mark the removed entry as invalid in the sparse set.
                self.sparse[key] = Self::INVALID_KEY;

                // Update the sparse index for the swapped entry.
                if dense_idx < self.dense.len() {
                    let swapped_key = &self.dense[dense_idx].key;
                    self.sparse[*swapped_key] = dense_idx;
                }
            } else {
                dense_idx += 1;
            }
        }

        removed.into_iter()
    }
}
