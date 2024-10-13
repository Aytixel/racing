use std::{
    cell::Cell,
    collections::LinkedList,
    fmt,
    future::{poll_fn, Future},
    sync::{
        mpsc::{RecvError, RecvTimeoutError, TryRecvError},
        Arc,
    },
    task::Poll,
    time::{Duration, Instant},
};

use crate::sync::Mutex;

pub struct Receiver<T> {
    queue: Arc<Mutex<LinkedList<T>>>,
}

unsafe impl<T: Send> Send for Receiver<T> {}

impl<T> Receiver<T> {
    pub(super) fn new(queue: Arc<Mutex<LinkedList<T>>>) -> Receiver<T> {
        Receiver { queue }
    }

    pub async fn recv(&self) -> Result<T, RecvError> {
        let mut queue = Cell::new(Box::pin(self.queue.lock()));

        poll_fn(|context| {
            let Poll::Ready(mut queue_) = queue.get_mut().as_mut().poll(context) else {
                return Poll::Pending;
            };

            queue.set(Box::pin(self.queue.lock()));

            if Arc::strong_count(&self.queue) == 1 {
                return Poll::Ready(Err(RecvError));
            }

            if let Some(value) = queue_.pop_front() {
                Poll::Ready(Ok(value))
            } else {
                Poll::Pending
            }
        })
        .await
    }

    pub async fn recv_timeout(&self, timeout: Duration) -> Result<T, RecvTimeoutError> {
        let mut queue = Cell::new(Box::pin(self.queue.lock()));
        let instant = Instant::now();

        poll_fn(|context| {
            let Poll::Ready(mut queue_) = queue.get_mut().as_mut().poll(context) else {
                return Poll::Pending;
            };

            queue.set(Box::pin(self.queue.lock()));

            if Arc::strong_count(&self.queue) == 1 {
                return Poll::Ready(Err(RecvTimeoutError::Disconnected));
            }

            if instant.elapsed() >= timeout {
                return Poll::Ready(Err(RecvTimeoutError::Timeout));
            }

            if let Some(value) = queue_.pop_front() {
                Poll::Ready(Ok(value))
            } else {
                Poll::Pending
            }
        })
        .await
    }

    pub async fn try_recv(&self) -> Result<T, TryRecvError> {
        if Arc::strong_count(&self.queue) == 1 {
            return Err(TryRecvError::Disconnected);
        }

        if let Some(value) = self.queue.lock().await.pop_front() {
            Ok(value)
        } else {
            Err(TryRecvError::Empty)
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for Receiver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Receiver").finish()
    }
}
