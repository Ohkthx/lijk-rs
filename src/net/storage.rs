use std::{collections::HashMap, time::Instant};

use crate::utils::SparseSet;

use super::{EntityId, SequenceId};

type Result<T> = std::result::Result<T, StorageError>;

/// Error types for the client storage.
#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub(crate) enum StorageError {
    OffsetOverflow,           // Offset overflow when creating the storage.
    InvalidClientIdCollision, // Invalid client ID collision when creating the storage.
    AtCapacity,               // Storage is at capacity when adding a new client.
    ClientExists,             // Client already exists in the storage.
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::OffsetOverflow => write!(f, "offset overflow"),
            StorageError::InvalidClientIdCollision => write!(f, "invalid client ID collision"),
            StorageError::AtCapacity => write!(f, "capacity reached"),
            StorageError::ClientExists => write!(f, "client already exists"),
        }
    }
}

/// Cache structure to store values with a timeout.
struct Cache<T> {
    timeout_ms: u128,              // Timeout in milliseconds for cache entries.
    lookup: HashMap<T, usize>,     // HashMap to store the lookup for fast access.
    data: SparseSet<(T, Instant)>, // Sparse set to store the cached data and their timestamps.
}

impl<T> Cache<T>
where
    T: Eq + std::hash::Hash + Clone + Copy,
{
    /// Initializes a new cache with a specified timeout.
    fn new(max_size: usize, timeout_ms: u128, invalid_key: usize) -> Self {
        Self {
            timeout_ms,
            lookup: HashMap::with_capacity(max_size),
            data: SparseSet::new(max_size, invalid_key),
        }
    }

    /// Looks up a value in the cache and returns its key if it exists.
    fn lookup(&self, value: &T) -> Option<usize> {
        self.lookup.get(value).copied()
    }

    /// Retrieves a value from the cache by key.
    #[allow(dead_code)]
    fn get(&self, key: usize) -> Option<&T> {
        self.data.get(key).map(|(data, _)| data)
    }

    /// Inserts a value into the cache with the current timestamp.
    fn insert(&mut self, key: usize, value: T) {
        self.lookup.insert(value, key);
        self.data.insert(key, (value, Instant::now()));
    }

    /// Removes a value from cache by key and returns the value if it exists.
    fn remove(&mut self, key: usize) -> Option<T> {
        if let Some(value) = self.data.remove(key).map(|(data, _)| data) {
            self.lookup.remove(&value);
            return Some(value);
        }

        None
    }

    /// Clears the cache return expired entries.
    fn drain(&mut self) -> Vec<(usize, T)> {
        let expired = self
            .data
            .drain_if(|(_, timestamp)| timestamp.elapsed().as_millis() >= self.timeout_ms)
            .map(|(client_id, (data, _))| {
                self.lookup.remove(&data);
                (client_id, data)
            })
            .collect();

        expired
    }
}

/// Information about the clients connected to the server.
pub(crate) struct ClientStorage<T> {
    id_offset: EntityId,   // Offset to add to the client ID.
    max_clients: usize,    // Maximum number of clients.
    invalid_key: EntityId, // Invalid key for the sparse set.

    addr_id: HashMap<T, usize>,      // Maps socket address to ID.
    addr: SparseSet<T>,              // Maps ID to socket address.
    sequence: SparseSet<SequenceId>, // Maps ID to sequence number.

    archive: Cache<T>, // Cache for archiving clients.

    pool: Vec<usize>, // Pool of IDs to use for new clients.
}

impl<T> ClientStorage<T>
where
    T: Eq + std::hash::Hash + Clone + Copy,
{
    /// Initializes the client information storage.
    pub fn new(id_offset: EntityId, max_clients: EntityId, invalid_key: EntityId) -> Result<Self> {
        if id_offset.checked_add(max_clients).is_none() {
            // Ensures Client ID returned is always valid.
            return Err(StorageError::OffsetOverflow);
        } else if invalid_key >= id_offset && invalid_key < id_offset + max_clients {
            // Ensures the invalid key does not overlap with valid client IDs.
            return Err(StorageError::InvalidClientIdCollision);
        }

        let max_clients = usize::from(max_clients);

        Ok(Self {
            id_offset,
            max_clients,
            invalid_key,

            addr_id: HashMap::with_capacity(max_clients),
            addr: SparseSet::new(max_clients, usize::from(invalid_key)),
            sequence: SparseSet::new(max_clients, usize::from(invalid_key)),

            archive: Cache::new(max_clients, 30000, usize::from(invalid_key)),

            pool: Vec::with_capacity(max_clients),
        })
    }

    /// Invalid client ID.
    #[inline]
    pub fn invalid_client(&self) -> EntityId {
        self.invalid_key
    }

    /// Maps an external ID to an internal ID.
    #[inline]
    fn map_internal(&self, id: EntityId) -> usize {
        usize::from(id) - usize::from(self.id_offset)
    }

    /// Maps an internal ID to an external ID.
    #[inline]
    fn map_external(&self, id: usize) -> EntityId {
        assert!(
            id <= usize::from(self.invalid_client()),
            "ID is out of bounds when mapping to external."
        );

        EntityId::try_from(id).unwrap() + self.id_offset
    }

    /// Runs maintanance on the cache to remove expired entries.
    pub fn run_tasks(&mut self) {
        for (client_id, _addr) in self.archive.drain() {
            // Add the ID back to the pool for reuse.
            self.pool.push(client_id);
        }
    }

    /// Checks if a client is in the archive. Returns an internal ID for the client.
    fn in_archive(&self, addr: &T) -> Option<usize> {
        self.archive.lookup(addr)
    }

    /// Obtains the sequence number for a client.
    pub fn get_sequence(&self, client_id: EntityId) -> Option<&SequenceId> {
        self.sequence.get(self.map_internal(client_id))
    }

    /// Obtains a mutable reference for the sequence number of a client.
    pub fn get_sequence_mut(&mut self, client_id: EntityId) -> Option<&mut SequenceId> {
        self.sequence.get_mut(self.map_internal(client_id))
    }

    /// Obtains the address from a clients ID.
    pub fn get_addr(&self, client_id: EntityId) -> Option<&T> {
        self.addr.get(self.map_internal(client_id))
    }

    /// Obtains the ID from a clients address.
    pub fn get_id(&self, addr: &T) -> Option<EntityId> {
        self.addr_id.get(addr).map(|id| self.map_external(*id))
    }

    /// Queues a client for removal by archiving its address.
    pub fn archive_client(&mut self, client_id: EntityId) {
        if let Some(addr) = self.remove(client_id) {
            self.archive.insert(self.map_internal(client_id), addr);
        }
    }

    /// Removes a client.
    fn remove(&mut self, client_id: EntityId) -> Option<T> {
        if let Some(addr) = self.addr.remove(self.map_internal(client_id)) {
            self.addr_id.remove(&addr);
            self.sequence.remove(self.map_internal(client_id));
            return Some(addr);
        }

        None
    }

    /// Inserts a client into the storage.
    pub fn insert(&mut self, client_id: EntityId, addr: T) {
        self.addr_id.insert(addr, self.map_internal(client_id));
        self.addr.insert(self.map_internal(client_id), addr);
        self.sequence.insert(self.map_internal(client_id), 0);
    }

    /// Adds a client to the storage. Returns the Client ID assigned.
    /// Returns `Self::INVALID_CLIENT_ID` if the maximum number of clients has been reached.
    pub fn add(&mut self, addr: T) -> Result<EntityId> {
        #[cfg(not(feature = "shared_ip"))]
        if self.addr_id.contains_key(&addr) || self.archive.lookup(&addr).is_some() {
            return Err(StorageError::ClientExists); // Client already exists.
        }

        #[cfg(feature = "shared_ip")]
        if let Some(id) = self.addr_id.get(&addr) {
            return Ok(self.map_external(*id)); // Client already exists.
        }

        let internal_id = if let Some(id) = self.in_archive(&addr) {
            self.archive.remove(id); // Reuse an ID from the archive.
            id
        } else if let Some(id) = self.pool.pop() {
            id // Reuse an ID form the pool.
        } else {
            self.addr.length() // Current length of the sparse set.
        };

        if internal_id >= self.max_clients && self.pool.is_empty() {
            return Err(StorageError::AtCapacity);
        }

        let client_id = self.map_external(internal_id);
        self.insert(client_id, addr);
        Ok(client_id)
    }

    /// Obtains the IDs and Socket Addresses of all clients.
    pub fn addr_iter(&self) -> impl Iterator<Item = (EntityId, &T)> + '_ {
        self.addr
            .iter()
            .map(|(id, addr)| (self.map_external(*id), addr))
    }

    /// Obtains the next ID to use for a new client.
    #[allow(dead_code)]
    pub fn next_id(&self) -> EntityId {
        if let Some(id) = self.pool.last() {
            self.map_external(*id)
        } else {
            self.map_external(self.addr.length())
        }
    }
}
