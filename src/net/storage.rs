use std::collections::HashMap;

use anyhow::{Result, bail};

use crate::{net::ConnectionError, utils::SparseSet};

/// Information about the clients connected to the server.
pub(crate) struct ClientStorage<T> {
    max_clients: u32,         // Maximum number of clients.
    id_offset: u32,           // Offset to add to the client ID.
    addr_id: HashMap<T, u32>, // Maps socket address to ID.
    addr: SparseSet<T>,       // Maps ID to socket address.
    sequence: SparseSet<u32>, // Maps ID to sequence number.
    pool: Vec<u32>,           // Pool of IDs to use for new clients.
}

impl<T> ClientStorage<T>
where
    T: Eq + std::hash::Hash + Clone + Copy,
{
    pub(crate) const INVALID_CLIENT_ID: u32 = SparseSet::<()>::INVALID_INDEX;

    /// Initializes the client information storage.
    pub(crate) fn new(max_clients: u32, id_offset: u32) -> Self {
        Self {
            max_clients,
            id_offset,
            addr_id: HashMap::with_capacity(max_clients as usize),
            addr: SparseSet::new(max_clients),
            sequence: SparseSet::new(max_clients),
            pool: Vec::with_capacity(max_clients as usize),
        }
    }

    /// Obtains the sequence number for a client.
    pub(crate) fn get_sequence(&self, client_id: u32) -> Option<&u32> {
        self.sequence.get(client_id - self.id_offset)
    }

    /// Obtains a mutable reference for the sequence number of a client.
    pub(crate) fn get_sequence_mut(&mut self, client_id: u32) -> Option<&mut u32> {
        self.sequence.get_mut(client_id - self.id_offset)
    }

    /// Obtains the address from a clients ID.
    pub(crate) fn get_addr(&self, client_id: u32) -> Option<&T> {
        self.addr.get(client_id - self.id_offset)
    }

    /// Obtains the ID from a clients address.
    pub(crate) fn get_id(&self, addr: &T) -> Option<u32> {
        self.addr_id.get(addr).map(|id| *id + self.id_offset)
    }

    /// Removes a client.
    pub(crate) fn remove(&mut self, client_id: u32) {
        if let Some(addr) = self.addr.remove(client_id - self.id_offset) {
            self.sequence.remove(client_id - self.id_offset);
            self.addr_id.remove(&addr);
            self.pool.push(client_id - self.id_offset);
        }
    }

    /// Inserts a client into the storage.
    pub(crate) fn insert(&mut self, client_id: u32, addr: T) {
        self.addr_id.insert(addr, client_id - self.id_offset);
        self.addr.insert(client_id - self.id_offset, addr);
        self.sequence.insert(client_id - self.id_offset, 0);
    }

    /// Adds a client to the storage. Returns the Client ID assigned.
    pub(crate) fn add(&mut self, addr: T) -> Result<u32> {
        let client_id = if let Some(id) = self.pool.pop() {
            // Reuse an ID from the pool.
            id
        } else {
            // Create a new ID.
            u32::try_from(self.addr.length()).expect("Could not convert usize to u32")
        };

        if (client_id == Self::INVALID_CLIENT_ID || client_id >= self.max_clients)
            && self.pool.is_empty()
        {
            bail!(ConnectionError::TooManyConnections)
        }

        self.addr_id.insert(addr, client_id);
        self.addr.insert(client_id, addr);
        self.sequence.insert(client_id, 0);
        Ok(client_id + self.id_offset)
    }

    /// Obtains the IDs and Socket Addresses of all clients.
    pub(crate) fn addr_iter(&self) -> impl Iterator<Item = (u32, &T)> + '_ {
        self.addr
            .iter()
            .map(|(id, addr)| (*id + self.id_offset, addr))
    }

    /// Obtains the next ID to use for a new client.
    pub(crate) fn next_id(&self) -> u32 {
        if let Some(id) = self.pool.last() {
            *id + self.id_offset
        } else {
            u32::try_from(self.addr.length()).expect("Could not convert usize to u32")
                + self.id_offset
        }
    }
}
