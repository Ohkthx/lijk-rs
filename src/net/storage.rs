use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::utils::SparseSet;

use super::ClientId;

type Result<T> = std::result::Result<T, StorageError>;

/// Error types for the client storage.
#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub(crate) enum StorageError {
    OffsetOverflow,           // Offset overflow when creating the storage.
    InvalidClientIdCollision, // Invalid client ID collision when creating the storage.
    AtCapacity,               // Storage is at capacity when adding a new client.
    ClientExists,             // Client already exists in the storage.
    TimedOut,                 // Client timed out.
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::OffsetOverflow => write!(f, "offset overflow"),
            StorageError::InvalidClientIdCollision => write!(f, "invalid client ID collision"),
            StorageError::AtCapacity => write!(f, "capacity reached"),
            StorageError::ClientExists => write!(f, "client already exists"),
            StorageError::TimedOut => write!(f, "client timed out"),
        }
    }
}

/// Information about the clients connected to the server.
pub(crate) struct ClientStorage<T> {
    id_offset: ClientId,   // Offset to add to the client ID.
    max_clients: usize,    // Maximum number of clients.
    invalid_key: ClientId, // Invalid key for the sparse set.

    addr_id: HashMap<T, usize>, // Maps socket address to ID.
    addr: SparseSet<T>,         // Maps ID to socket address.
    sequence: SparseSet<u16>,   // Maps ID to sequence number.
    ping: SparseSet<Instant>,   // Maps ID to ping.

    archive: HashMap<T, (usize, Instant)>, // Cache for archiving clients.
    errors: HashMap<T, (usize, Instant)>,  // Cache for error counts.
    blacklist: HashMap<T, Instant>,        // Blacklist for clients.

    pool: Vec<usize>, // Pool of IDs to use for new clients.
}

impl<T> ClientStorage<T>
where
    T: Eq + std::hash::Hash + Clone + Copy,
{
    /// Initializes the client information storage.
    pub fn new(id_offset: ClientId, max_clients: ClientId, invalid_key: ClientId) -> Result<Self> {
        if id_offset.0.checked_add(max_clients.0).is_none() {
            // Ensures Client ID returned is always valid.
            return Err(StorageError::OffsetOverflow);
        } else if invalid_key >= id_offset && invalid_key.0 < id_offset.0 + max_clients.0 {
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
            ping: SparseSet::new(max_clients, usize::from(invalid_key)),

            // archive: Cache::new(max_clients, usize::from(invalid_key)),
            archive: HashMap::new(),
            errors: HashMap::new(),
            blacklist: HashMap::new(),

            pool: Vec::with_capacity(max_clients),
        })
    }

    /// Invalid client ID.
    #[inline]
    pub fn invalid_client(&self) -> ClientId {
        self.invalid_key
    }

    /// Maps an external ID to an internal ID.
    #[inline]
    fn map_internal(&self, id: ClientId) -> usize {
        usize::from(id) - usize::from(self.id_offset)
    }

    /// Maps an internal ID to an external ID.
    #[inline]
    fn map_external(&self, id: usize) -> ClientId {
        assert!(
            id <= usize::from(self.invalid_client()),
            "ID is out of bounds when mapping to external."
        );

        ClientId(ClientId::try_from(id).unwrap().0 + self.id_offset.0)
    }

    /// Drains the archive of expired entries and returns them to the pool.
    pub fn task_drain_archive(&mut self, drain_ms: u64) {
        let mut expired = vec![];
        self.archive.retain(|_, (client_id, timestamp)| {
            // Retain only the entries that are not expired.
            if timestamp.elapsed().as_millis() < u128::from(drain_ms) {
                true
            } else {
                expired.push(*client_id);
                false
            }
        });

        for client_id in expired {
            self.pool.push(client_id); // Add the ID back to the pool for reuse.
        }
    }

    /// Drains the blacklist cache of expired entries. This will remove clients that have been timed out.
    pub fn task_drain_blacklist(&mut self, timeout_ms: u64) {
        if !self.blacklist.is_empty() {
            self.blacklist.retain(|_addr, timestamp| {
                timestamp.elapsed().as_millis() < u128::from(timeout_ms)
            });
        }
    }

    /// Resets the errors cache to remove expired entries.
    pub fn task_reset_errors(&mut self, errors_ms: u64) {
        // Drain the errors cache to remove expired entries.
        if !self.errors.is_empty() {
            self.errors.retain(|_addr, (_count, timestamp)| {
                timestamp.elapsed().as_millis() < u128::from(errors_ms)
            });
        }
    }

    /// Checks if a client is currently timed out.
    pub fn is_blacklisted(&self, addr: &T) -> bool {
        self.blacklist.contains_key(addr)
    }

    /// Obtains the sequence number for a client.
    pub fn get_sequence(&self, client_id: ClientId) -> Option<&u16> {
        self.sequence.get(self.map_internal(client_id))
    }

    /// Obtains a mutable reference for the sequence number of a client.
    pub fn get_sequence_mut(&mut self, client_id: ClientId) -> Option<&mut u16> {
        self.sequence.get_mut(self.map_internal(client_id))
    }

    /// Obtains the ping for a client.
    #[allow(dead_code)]
    pub fn get_ping(&self, client_id: ClientId) -> Option<&Instant> {
        self.ping.get(self.map_internal(client_id))
    }

    /// Obtains a mutable reference for the ping of a client.
    #[allow(dead_code)]
    pub fn get_ping_mut(&mut self, client_id: ClientId) -> Option<&mut Instant> {
        self.ping.get_mut(self.map_internal(client_id))
    }

    /// Obtains the error count for a client.
    pub fn get_errors(&mut self, addr: &T) -> Option<&usize> {
        self.errors.get(addr).map(|(count, _)| count)
    }

    /// Adds an error to a client. Creates it if the client does not exist.
    pub fn client_err(&mut self, addr: T) {
        if let Some((count, timestamp)) = self.errors.get_mut(&addr) {
            *timestamp = Instant::now();
            *count += 1;
        } else {
            self.errors.insert(addr, (1, Instant::now()));
        }
    }

    /// Obtains the address from a clients ID.
    pub fn get_addr(&self, client_id: ClientId) -> Option<&T> {
        self.addr.get(self.map_internal(client_id))
    }

    /// Obtains the ID from a clients address.
    pub fn get_id(&self, addr: &T) -> Option<ClientId> {
        self.addr_id.get(addr).map(|id| self.map_external(*id))
    }

    /// Queues a client for removal by archiving its address.
    pub fn archive_client(&mut self, client_id: ClientId) {
        if let Some(addr) = self.remove(client_id) {
            self.archive
                .insert(addr, (self.map_internal(client_id), Instant::now()));
        }
    }

    ///  Blacklists a client and allows its `ClientId` to be reused.
    pub fn blacklist_client(&mut self, client_id: ClientId, addr: &T) {
        if let Some(addr) = self.remove(client_id) {
            self.blacklist.insert(addr, Instant::now());
            self.pool.push(self.map_internal(client_id));
        } else if self.archive.remove(addr).is_some() {
            self.blacklist.insert(*addr, Instant::now());
            self.pool.push(self.map_internal(client_id));
        }
    }

    /// Blacklists a client by its address.
    pub fn blacklist_client_addr(&mut self, addr: &T) {
        if let Some(client_id) = self.addr_id.get(addr) {
            self.blacklist_client(self.map_external(*client_id), addr);
        } else if let Some((client_id, _)) = self.archive.get(addr) {
            self.blacklist_client(self.map_external(*client_id), addr);
        } else {
            self.blacklist.insert(*addr, Instant::now());
        }
    }

    /// Returns a list of clients that have timed out based on the specified timeout.
    pub fn expired_clients(&self, timeout_ms: u64) -> Vec<ClientId> {
        let now = Instant::now();
        self.ping
            .iter()
            .filter_map(|(client_id, timestamp)| {
                if now.duration_since(*timestamp) > Duration::from_millis(timeout_ms) {
                    Some(self.map_external(*client_id))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Removes a client.
    fn remove(&mut self, client_id: ClientId) -> Option<T> {
        if let Some(addr) = self.addr.remove(self.map_internal(client_id)) {
            self.addr_id.remove(&addr);
            self.sequence.remove(self.map_internal(client_id));
            self.ping.remove(self.map_internal(client_id));
            return Some(addr);
        }

        None
    }

    /// Inserts a client into the storage.
    pub fn insert(&mut self, client_id: ClientId, addr: T) {
        self.addr_id.insert(addr, self.map_internal(client_id));
        self.addr.insert(self.map_internal(client_id), addr);
        self.sequence.insert(self.map_internal(client_id), 0);
        self.ping
            .insert(self.map_internal(client_id), Instant::now());
    }

    /// Adds a client to the storage. Returns the Client ID assigned.
    /// Returns `Self::INVALID_CLIENT_ID` if the maximum number of clients has been reached.
    pub fn add(&mut self, addr: T) -> Result<ClientId> {
        if self.is_blacklisted(&addr) {
            return Err(StorageError::TimedOut); // Client timed out.
        }

        #[cfg(not(feature = "shared_ip"))]
        if self.addr_id.contains_key(&addr) || self.archive.contains_key(&addr) {
            return Err(StorageError::ClientExists); // Client already exists.
        }

        #[cfg(feature = "shared_ip")]
        if let Some(id) = self.addr_id.get(&addr) {
            return Ok(self.map_external(*id)); // Client already exists.
        }

        let internal_id = if let Some((id, _)) = self.archive.remove(&addr) {
            id // Reuse an ID from the archive.
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
    pub fn addr_iter(&self) -> impl Iterator<Item = (ClientId, &T)> + '_ {
        self.addr
            .iter()
            .map(|(id, addr)| (self.map_external(*id), addr))
    }

    /// Obtains the next ID to use for a new client.
    #[allow(dead_code)]
    pub fn next_id(&self) -> ClientId {
        if let Some(id) = self.pool.last() {
            self.map_external(*id)
        } else {
            self.map_external(self.addr.length())
        }
    }
}
