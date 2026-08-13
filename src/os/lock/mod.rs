macro_rules! x {
    ($a:literal $b:ident) => {
        #[cfg(target_os = $a)]
        mod $b;
        #[cfg(target_os = $a)]
        pub use $b::*;
    };
}

x!("linux" linux);
