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
    max_capacity: usize,  // Maximum capacity of the sparse set.
    invalid_key: usize,   // Invalid index for the sparse set.
}

impl<T> SparseSet<T> {
    /// Creates a new sparse set with the given capacity.
    pub fn new(capacity: usize, invalid_key: usize) -> Self {
        assert!(
            capacity <= invalid_key,
            "Capacity must be less than or equal to invalid_key"
        );

        Self {
            dense: Vec::with_capacity(capacity),
            sparse: vec![invalid_key; capacity],
            max_capacity: capacity,
            invalid_key,
        }
    }

    /// Invalid key for the sparse set.
    #[inline]
    pub fn invalid_key(&self) -> usize {
        self.invalid_key
    }

    /// Obtains the dense index for the key provided.
    fn get_dense_idx(&self, key: usize) -> Option<usize> {
        if key >= self.max_capacity {
            return None;
        }

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

    /// Inserts a new value at the specified index in the sparse set. Returns the dense index
    /// if the insertion is successful, or an error if the index is out of bounds.
    pub fn insert(&mut self, key: usize, value: T) -> usize {
        assert!(
            key < self.max_capacity,
            "Key out of bounds: {key} >= {}",
            self.max_capacity
        );

        if let Some(stored) = self.get_mut(key) {
            // Index already in dense set.
            *stored = value;
        } else {
            // Index not in dense set.
            self.sparse[key] = self.dense.len();
            self.dense.push(Entry { key, value });
        }

        self.sparse[key]
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

        self.sparse[key] = self.invalid_key();
        Some(entry.value)
    }

    /// Allows for iteration over the dense set.
    pub fn iter(&self) -> SparseSetIterator<T> {
        SparseSetIterator {
            sset: self,
            index: 0,
        }
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
                self.sparse[key] = self.invalid_key();

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

/// Simple iterator for the sparse set.
pub struct SparseSetIterator<'a, T> {
    sset: &'a SparseSet<T>,
    index: usize,
}

impl<'a, T> Iterator for SparseSetIterator<'a, T> {
    type Item = (&'a usize, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        if (self.index) < self.sset.dense.len() {
            let entry = &self.sset.dense[self.index];
            self.index += 1;
            Some((&entry.key, &entry.value))
        } else {
            None
        }
    }
}
