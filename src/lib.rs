use std::pin::Pin;

pub mod runtime;
pub mod task;

pub(crate) type BoxFuture<'a, T> = Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;
