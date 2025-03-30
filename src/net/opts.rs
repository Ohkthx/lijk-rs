pub struct SocketOptions {
    /// The maximum number of clients that can be connected to this socket.
    pub(crate) max_clients: u16,
    /// Address of the remote server. None to set the socket as a server.
    pub(crate) server_address: Option<String>,
    /// Interval for the task scheduler to check all tasks.
    pub(crate) task_interval_ms: u64,
    /// Interval for clearing archived clients. None to never clear.
    pub(crate) archive_interval_ms: Option<u64>,
    /// Interval for clearing blacklisted clients. None to never clear.
    pub(crate) blacklist_interval_ms: Option<u64>,
    /// Interval for clearing error counts for clients. None to never clear.
    pub(crate) error_reset_interval_ms: Option<u64>,
    /// Interval for disconnecting clients or from host.
    pub(crate) disconnect_interval_ms: Option<u64>,
    /// Interval for sending ping packets.
    pub(crate) ping_interval_ms: Option<u64>,
}

#[allow(dead_code)]
impl SocketOptions {
    /// Default addresses for the server.
    pub(crate) const DEFAULT_SERVER_ADDR: &'static str = "127.0.0.1:31013";
    /// Default address for the client to bind to. This is used when the client does not have a specific address.
    pub(crate) const DEFAULT_CLIENT_ADDR: &'static str = "0.0.0.0:0";

    /// Default options for a client socket.
    pub fn default_client() -> Self {
        SocketOptions {
            max_clients: 1,
            server_address: Some(Self::DEFAULT_SERVER_ADDR.to_string()),
            task_interval_ms: 5000,
            archive_interval_ms: None,
            blacklist_interval_ms: None,
            error_reset_interval_ms: None,
            disconnect_interval_ms: Some(15000),
            ping_interval_ms: Some(5000),
        }
    }

    /// Default options for a server socket.
    pub fn default_server() -> Self {
        SocketOptions {
            max_clients: 256,
            server_address: None,
            task_interval_ms: 1000,
            archive_interval_ms: Some(30000),
            blacklist_interval_ms: Some(30000),
            error_reset_interval_ms: Some(60000),
            disconnect_interval_ms: Some(15000),
            ping_interval_ms: None,
        }
    }

    /// Creates a new `SocketOptions` instance based on whether it is a server or client.
    pub fn new(is_server: bool) -> Self {
        if is_server {
            Self::default_server()
        } else {
            Self::default_client()
        }
    }

    // Returns true if the socket is configured as a server.
    pub fn is_server(&self) -> bool {
        self.server_address.is_none()
    }

    /// Sets the maximum number of clients that can be connected to this socket.
    pub fn max_clients(mut self, max_clients: u16) -> Self {
        self.max_clients = max_clients;
        self
    }

    /// Sets the server address for the socket.
    pub fn server_address<N: Into<String>>(mut self, address: N) -> Self {
        self.server_address = Some(address.into());
        self
    }

    /// Sets the interval in which the task scheduler will check all tasks in milliseconds.
    pub fn task_interval(mut self, interval_ms: u64) -> Self {
        // Sets the interval for the task scheduler to check all tasks
        self.task_interval_ms = interval_ms;
        self
    }

    /// Sets the interval for clearing archived clients in milliseconds.
    pub fn archive_interval(mut self, interval_ms: u64) -> Self {
        self.archive_interval_ms = Some(interval_ms);
        self
    }

    /// Disables the archive clearing task.
    pub fn disable_archive(mut self) -> Self {
        // Disables the archive interval by setting it to None
        self.archive_interval_ms = None;
        self
    }

    /// Sets the interval for clearing blacklisted clients in milliseconds.
    pub fn blacklist_interval(mut self, interval_ms: u64) -> Self {
        self.blacklist_interval_ms = Some(interval_ms);
        self
    }

    /// Disables the blacklist clearing task.
    pub fn disable_blacklist(mut self) -> Self {
        // Disables the blacklist interval by setting it to None
        self.blacklist_interval_ms = None;
        self
    }

    /// Sets the interval for resetting error counts for clients in milliseconds.
    pub fn error_reset_interval(mut self, interval_ms: u64) -> Self {
        self.error_reset_interval_ms = Some(interval_ms);
        self
    }

    /// Disables the error reset interval.
    pub fn disable_error_reset(mut self) -> Self {
        // Disables the error reset interval by setting it to None
        self.error_reset_interval_ms = None;
        self
    }

    /// Sets the interval for disconnecting clients or from host in milliseconds.
    pub fn disconnect_interval(mut self, interval_ms: u64) -> Self {
        self.disconnect_interval_ms = Some(interval_ms);
        self
    }

    /// Disables the disconnect interval.
    pub fn disable_disconnect(mut self) -> Self {
        // Disables the disconnect interval by setting it to None
        self.disconnect_interval_ms = None;
        self
    }

    /// Sets the interval for sending ping packets in milliseconds.
    pub fn ping_interval(mut self, interval_ms: u64) -> Self {
        self.ping_interval_ms = Some(interval_ms);
        self
    }

    /// Disables the ping interval.
    pub fn disable_ping(mut self) -> Self {
        // Disables the ping interval by setting it to None
        self.ping_interval_ms = None;
        self
    }
}
