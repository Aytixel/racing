use std::pin::Pin;

pub mod io;
pub mod net;
pub mod runtime;
pub mod sync;
pub mod thread;

pub(crate) type BoxFuture<'a, T> = Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;
