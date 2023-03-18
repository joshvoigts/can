#[macro_export]
macro_rules! fail {
    ($($vars:expr),*) => {
        {
            println!($($vars),*);
            process::exit(1);
        }
    };
}
