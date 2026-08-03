/// Produces the value of TARGET as a string literal.
#[macro_export]
macro_rules! target {
    () => {
        "x86_64-pc-windows-msvc"
    };
}

/// Produces the value of HOST as a string literal.
#[macro_export]
macro_rules! host {
    () => {
        "x86_64-pc-windows-msvc"
    };
}
