#[macro_export]
macro_rules! p {
    ($($opt:expr),*) => {
        {
            $(
                print!("{:?} ", $opt);
             )*
        }
        println!();
    };
}

// macro_rules! hashmap(
//     { $($key:expr => $value:expr),+ } => {
//         {
//             let mut m = ::std::collections::HashMap::new();
//             $(
//                 m.insert($key, $value);
//             )+
//             m
//         }
//      };
// );
