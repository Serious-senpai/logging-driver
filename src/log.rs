#[macro_export]
macro_rules! log {
    () => {
        wdk::println!("[LoggingDriver] \n");
    };
    ($($arg:tt)*) => {
        wdk::println!("{}\n", format_args!($($arg)*));
    };
}
