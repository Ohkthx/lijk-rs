#[derive(Debug)]
struct Entry<T> {
    key: u32,
    value: T,
}

/// A sparse set is a data structure that allows for efficient insertion, deletion, and lookup of
#[derive(Debug)]
pub struct SparseSet<T> {
    dense: Vec<Entry<T>>, // Dense set of values.
    sparse: Vec<u32>,     // Sparse set of indices.
    max_capacity: u32,    // Maximum capacity of the sparse set.
}

impl<T> SparseSet<T> {
    /// Invalid index for the sparse set.
    pub const INVALID_INDEX: u32 = u32::MAX;

    /// Creates a new sparse set with the given capacity.
    pub fn new(capacity: u32) -> Self {
        Self {
            dense: Vec::with_capacity(capacity as usize),
            sparse: vec![Self::INVALID_INDEX; capacity as usize],
            max_capacity: capacity,
        }
    }

    /// Obtains the dense index for the key provided.
    fn get_dense_idx(&self, key: u32) -> Option<u32> {
        if key >= self.max_capacity {
            return None;
        }

        let dense_idx = self.sparse[key as usize];
        if (dense_idx as usize) < self.dense.len() {
            Some(dense_idx)
        } else {
            None
        }
    }

    /// Checks if the sparse set contains the key.
    pub fn has_key(&self, key: u32) -> bool {
        self.get_dense_idx(key).is_some()
    }

    /// Gets the amount of elements in the sparse set.
    pub fn length(&self) -> usize {
        self.dense.len()
    }

    /// Obtains a reference for the value associated with the key.
    pub fn get(&self, key: u32) -> Option<&T> {
        if let Some(dense_idx) = self.get_dense_idx(key) {
            Some(&self.dense[dense_idx as usize].value)
        } else {
            None
        }
    }

    /// Obtains a mutable reference for the value associated with the key.
    pub fn get_mut(&mut self, key: u32) -> Option<&mut T> {
        if let Some(dense_idx) = self.get_dense_idx(key) {
            Some(&mut self.dense[dense_idx as usize].value)
        } else {
            None
        }
    }

    /// Inserts a new value at the specified index in the sparse set. Returns the dense index
    /// if the insertion is successful, or an error if the index is out of bounds.
    pub fn insert(&mut self, key: u32, value: T) -> u32 {
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
            self.sparse[key as usize] =
                u32::try_from(self.dense.len()).expect("Could not convert usize to u32");
            self.dense.push(Entry { key, value });
        }

        self.sparse[key as usize]
    }

    /// Removes a value based on the key provided.
    pub fn remove(&mut self, key: u32) -> Option<T> {
        if !self.has_key(key) {
            return None;
        }

        let dense_idx = self.sparse[key as usize];
        let entry = self.dense.swap_remove(dense_idx as usize);
        if (dense_idx as usize) < self.dense.len() {
            let swapped = &self.dense[dense_idx as usize];
            self.sparse[swapped.key as usize] = dense_idx;
        }

        self.sparse[key as usize] = Self::INVALID_INDEX;
        Some(entry.value)
    }

    /// Allows for iteration over the dense set.
    pub fn iter(&self) -> SparseSetIterator<T> {
        SparseSetIterator {
            sset: self,
            index: 0,
        }
    }
}

/// Simple iterator for the sparse set.
pub struct SparseSetIterator<'a, T> {
    sset: &'a SparseSet<T>,
    index: u32,
}

impl<'a, T> Iterator for SparseSetIterator<'a, T> {
    type Item = (&'a u32, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        if (self.index as usize) < self.sset.dense.len() {
            let entry = &self.sset.dense[self.index as usize];
            self.index += 1;
            Some((&entry.key, &entry.value))
        } else {
            None
        }
    }
}
