#[macro_export]
macro_rules! parallel
{
    { in $n:expr => $($tt:tt)* } =>
    {
        {
            let mut pool = vec![];

            for _ in 0..$n
            {
                pool.push(spawn(|| { $($tt)* }));
            }

            pool
        }
    }
}
