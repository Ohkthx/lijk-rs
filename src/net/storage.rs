use std::collections::HashMap;

use crate::utils::SparseSet;

use super::{EntityId, SequenceId};

type Result<T> = std::result::Result<T, StorageError>;

/// Error types for the client storage.
#[derive(Debug, PartialEq)]
pub(crate) enum StorageError {
    OffsetOverflow,           // Offset overflow when creating the storage.
    InvalidClientIdCollision, // Invalid client ID collision when creating the storage.
    AtCapacity,               // Storage is at capacity when adding a new client.
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::OffsetOverflow => write!(f, "offset overflow"),
            StorageError::InvalidClientIdCollision => write!(f, "invalid client ID collision"),
            StorageError::AtCapacity => write!(f, "capacity reached"),
        }
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

    pool: Vec<usize>, // Pool of IDs to use for new clients.
}

impl<T> ClientStorage<T>
where
    T: Eq + std::hash::Hash + Clone + Copy,
{
    /// Initializes the client information storage.
    pub(crate) fn new(
        id_offset: EntityId,
        max_clients: EntityId,
        invalid_key: EntityId,
    ) -> Result<Self> {
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

            pool: Vec::with_capacity(max_clients),
        })
    }

    /// Invalid client ID.
    #[inline]
    pub(crate) fn invalid_client(&self) -> EntityId {
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

    /// Obtains the sequence number for a client.
    pub(crate) fn get_sequence(&self, client_id: EntityId) -> Option<&SequenceId> {
        self.sequence.get(self.map_internal(client_id))
    }

    /// Obtains a mutable reference for the sequence number of a client.
    pub(crate) fn get_sequence_mut(&mut self, client_id: EntityId) -> Option<&mut SequenceId> {
        self.sequence.get_mut(self.map_internal(client_id))
    }

    /// Obtains the address from a clients ID.
    pub(crate) fn get_addr(&self, client_id: EntityId) -> Option<&T> {
        self.addr.get(self.map_internal(client_id))
    }

    /// Obtains the ID from a clients address.
    pub(crate) fn get_id(&self, addr: &T) -> Option<EntityId> {
        self.addr_id.get(addr).map(|id| self.map_external(*id))
    }

    /// Removes a client.
    pub(crate) fn remove(&mut self, client_id: EntityId) {
        if let Some(addr) = self.addr.remove(self.map_internal(client_id)) {
            self.addr_id.remove(&addr);
            self.sequence.remove(self.map_internal(client_id));

            // Add the ID back to the pool for reuse.
            self.pool.push(self.map_internal(client_id));
        }
    }

    /// Inserts a client into the storage.
    pub(crate) fn insert(&mut self, client_id: EntityId, addr: T) {
        self.addr_id.insert(addr, self.map_internal(client_id));
        self.addr.insert(self.map_internal(client_id), addr);
        self.sequence.insert(self.map_internal(client_id), 0);
    }

    /// Adds a client to the storage. Returns the Client ID assigned.
    /// Returns `Self::INVALID_CLIENT_ID` if the maximum number of clients has been reached.
    pub(crate) fn add(&mut self, addr: T) -> Result<EntityId> {
        let internal_id = if let Some(id) = self.pool.pop() {
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
    pub(crate) fn addr_iter(&self) -> impl Iterator<Item = (EntityId, &T)> + '_ {
        self.addr
            .iter()
            .map(|(id, addr)| (self.map_external(*id), addr))
    }

    /// Obtains the next ID to use for a new client.
    #[allow(dead_code)]
    pub(crate) fn next_id(&self) -> EntityId {
        if let Some(id) = self.pool.last() {
            self.map_external(*id)
        } else {
            self.map_external(self.addr.length())
        }
    }
}
