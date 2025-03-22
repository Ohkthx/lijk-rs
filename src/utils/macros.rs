/// Prints out a debug message, wraps `println!` macro.
#[macro_export]
macro_rules! debugln {
    ($fmt:expr $(, $($arg:tt)*)?) => {
        #[cfg(any(debug_assertions, feature = "debug_output"))]
        println!(concat!("[DEBUG] ", $fmt), $($($arg)*)?);
    };
}

/// Short form for returning an `Error` within a `Result` function.
#[macro_export]
macro_rules! flee {
    ($err:expr) => {
        return Err($err)
    };
}
