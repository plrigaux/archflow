use std::future::Future;
pub mod archive;
pub mod async_write_wrapper;
pub mod compression;
mod constants;
mod descriptor;
pub mod tools;
pub mod types;
trait AsyncFn<T, U, V>: Fn(T, U, V) -> <Self as AsyncFn<T, U, V>>::Fut {
    type Fut: Future<Output = <Self as AsyncFn<T, U, V>>::Output>;
    type Output;
}

impl<T, U, V, F, Fut> AsyncFn<T, U, V> for F
where
    F: Fn(T, U, V) -> Fut,
    Fut: Future,
{
    type Fut = Fut;
    type Output = Fut::Output;
}
