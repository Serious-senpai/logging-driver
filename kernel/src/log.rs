#[macro_export]
macro_rules! log {
    () => {
        wdk::println!("[LoggingDriver]");
    };
    ($($arg:tt)*) => {
        wdk::println!("[LoggingDriver] {}", format_args!($($arg)*));
    };
}
