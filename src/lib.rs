use std::future::Future;
pub mod archive;
pub mod async_write_wrapper;
mod compression;
mod constants;
pub mod tools;

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
