#![warn(clippy::pedantic)]

use std::fmt::Display;
use std::fmt::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use client::ClientCore;
use error::{AppError, Result};
use net::{Socket, SocketOptions};
use server::ServerCore;

mod client;
mod error;
mod net;
mod server;
mod shared;
mod utils;
mod vec2f;

const SERVER_TICK_RATE: u16 = 20;
const _CLIENT_TICK_RATE: u16 = 30;

enum Flags {
    Help,
    Remote,
    Local,
    Client,
    Server,
    Solo,
}

impl Flags {
    /// List of currently enabled valid flags for the application.
    const ENABLED: [Flags; 6] = [
        Flags::Help,
        Flags::Remote,
        Flags::Local,
        Flags::Client,
        Flags::Server,
        Flags::Solo,
    ];

    /// Creates the help message for the application.
    fn help() -> String {
        let mut header = String::from("Usage: cargo run -- ");
        for flag in &Flags::ENABLED {
            write!(&mut header, "[{flag}] ").unwrap();
        }

        header.push_str("\n\nOptions:");
        for flag in &Flags::ENABLED {
            write!(&mut header, "\n  {}", flag.description()).unwrap();
        }
        header
    }

    /// Returns the description of the flag.
    fn description(&self) -> String {
        match self {
            Flags::Help => String::from("--help: Show this help message."),
            Flags::Remote => String::from("--remote: Use a remote connection to the server."),
            Flags::Local => String::from("--local: Use a local connection to the server."),
            Flags::Client => String::from("--client: Run as a client."),
            Flags::Server => String::from("--server: Run as a server."),
            Flags::Solo => String::from("--solo: Run both client and server in the same process."),
        }
    }
}

impl From<&Flags> for &'static str {
    fn from(val: &Flags) -> Self {
        match val {
            Flags::Help => "--help",
            Flags::Remote => "--remote",
            Flags::Local => "--local",
            Flags::Client => "--client",
            Flags::Server => "--server",
            Flags::Solo => "--solo",
        }
    }
}

impl Display for Flags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", <&'static str>::from(self))
    }
}

/// Spawns a server and a client in separate threads.
fn as_solo(args: &[String]) -> std::result::Result<(), std::boxed::Box<dyn std::error::Error>> {
    let (sconn, cconn) = if args.contains(&Flags::Remote.to_string()) {
        // Initialize the remote connections.
        let server_opts = SocketOptions::default_server();
        let server = Socket::new_remote(&server_opts).map_err(AppError::Net)?;

        let client_opts = SocketOptions::default_client().server_address(server.addr());
        let client = Socket::new_remote(&client_opts).map_err(AppError::Net)?;

        (server, client)
    } else if args.contains(&Flags::Local.to_string()) {
        // Initialize the local connections.
        Socket::new_local_pair().map_err(AppError::Net)?
    } else {
        Socket::new_local_pair().map_err(AppError::Net)?
    };

    // Create a shutdown flag to signal the server to stop.
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let flag_clone = Arc::clone(&shutdown_flag);

    // Spawn the server with a connection in a separate thread.
    let server_run = std::thread::spawn(move || {
        let _ = ServerCore::new(sconn, Some(flag_clone)).run(SERVER_TICK_RATE);
    });

    // Create the client with a connection.
    let mut client = ClientCore::new(cconn)?;
    client.run()?;
    shutdown_flag.store(true, Ordering::Relaxed);

    server_run.join().expect("Server thread panicked.");
    Ok(())
}

/// Spawns a remote client used to connect to a remote server.
fn as_client() -> std::result::Result<(), std::boxed::Box<dyn std::error::Error>> {
    // Create a socket to connect to the server.
    let client_opts = SocketOptions::default_client();
    let socket = Socket::new_remote(&client_opts).map_err(AppError::Net)?;

    let mut client = ClientCore::new(socket)?;
    client.run()?;
    Ok(())
}

/// Spawns a server that clients can connect to.
fn as_server() -> std::result::Result<(), std::boxed::Box<dyn std::error::Error>> {
    let server_opts = SocketOptions::default_server();
    let socket = Socket::new_remote(&server_opts).map_err(AppError::Net)?;
    ServerCore::new(socket, None).run(SERVER_TICK_RATE)?;
    Ok(())
}

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let result = if args.contains(&Flags::Help.to_string()) {
        println!("{}", Flags::help());
        Ok(())
    } else if args.contains(&Flags::Client.to_string()) {
        as_client()
    } else if args.contains(&Flags::Server.to_string()) {
        as_server()
    } else if args.contains(&Flags::Solo.to_string()) {
        as_solo(&args)
    } else {
        println!("{}", Flags::help());
        Ok(())
    };

    if let Err(why) = result {
        println!("Error: {why}");
    } else {
        println!("Application exited successfully.");
    }
}
