use std::{
    future::{self, poll_fn, Future},
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
    time::{Duration, Instant},
};

use crate::{runtime::FutureQueue, BoxFuture};

enum PollHandle<T> {
    Ready(Option<T>),
    Pending(BoxFuture<'static, T>),
}

impl<T> PollHandle<T> {
    fn new(future: BoxFuture<'static, T>) -> Arc<Mutex<PollHandle<T>>> {
        Arc::new(Mutex::new(PollHandle::Pending(future)))
    }
}

pub struct JoinHandle<T>(Arc<Mutex<PollHandle<T>>>);

impl<T> Future for JoinHandle<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Self::Output> {
        let Ok(mut poll_handle) = self.0.try_lock() else {
            return Poll::Pending;
        };
        let PollHandle::Ready(result) = &mut *poll_handle else {
            return Poll::Pending;
        };

        Poll::Ready(result.take().unwrap())
    }
}

pub fn spawn<T, F>(future: F) -> JoinHandle<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let poll_handle = PollHandle::new(Box::pin(future));
    let poll_handle_clone = poll_handle.clone();
    let queue = FutureQueue::get_thread_local();

    queue.send(Box::pin(poll_fn(move |context| {
        let poll_handle = poll_handle_clone.clone();
        let Ok(mut poll_handle) = poll_handle.try_lock() else {
            return Poll::Pending;
        };
        let PollHandle::Pending(future) = &mut *poll_handle else {
            return Poll::Pending;
        };
        let Poll::Ready(result) = future.as_mut().poll(context) else {
            return Poll::Pending;
        };

        *poll_handle = PollHandle::Ready(Some(result));

        Poll::Ready(())
    })));

    JoinHandle(poll_handle)
}

pub async fn sleep(duration: Duration) {
    sleep_util(Instant::now() + duration).await
}

pub async fn sleep_util(instant: Instant) {
    poll_fn(|_context| {
        if instant.checked_duration_since(Instant::now()).is_none() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await;
}

pub async fn yield_now() {
    future::ready(()).await
}
