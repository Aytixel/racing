use std::{
    cell::Cell,
    collections::LinkedList,
    fmt,
    future::{poll_fn, Future},
    sync::{
        mpsc::{SendError, TrySendError},
        Arc,
    },
    task::Poll,
};

use crate::sync::Mutex;

#[derive(Clone)]
pub struct SyncSender<T> {
    queue: Arc<Mutex<LinkedList<T>>>,
    sender: Arc<()>,
    bound: usize,
}

unsafe impl<T: Send> Send for SyncSender<T> {}
unsafe impl<T: Send> Sync for SyncSender<T> {}

impl<T> SyncSender<T> {
    pub(super) fn new(queue: Arc<Mutex<LinkedList<T>>>, bound: usize) -> SyncSender<T> {
        SyncSender {
            queue,
            sender: Arc::new(()),
            bound,
        }
    }

    pub async fn send(&self, value: T) -> Result<(), SendError<T>> {
        let mut queue = Cell::new(Box::pin(self.queue.lock()));
        let mut value = Some(value);

        poll_fn(|context| {
            if let Some(value_) = value.take() {
                if Arc::strong_count(&self.queue) - Arc::strong_count(&self.sender) == 1 {
                    let Poll::Ready(mut queue_) = queue.get_mut().as_mut().poll(context) else {
                        return Poll::Pending;
                    };

                    queue.set(Box::pin(self.queue.lock()));

                    if self.bound >= queue_.len() {
                        value = Some(value_);
                        Poll::Pending
                    } else {
                        queue_.push_back(value_);
                        Poll::Ready(Ok(()))
                    }
                } else {
                    Poll::Ready(Err(SendError(value_)))
                }
            } else {
                Poll::Ready(Ok(()))
            }
        })
        .await
    }

    pub async fn try_send(&self, value: T) -> Result<(), TrySendError<T>> {
        if Arc::strong_count(&self.queue) - Arc::strong_count(&self.sender) == 1 {
            let mut queue = self.queue.lock().await;

            if self.bound >= queue.len() {
                Err(TrySendError::Full(value))
            } else {
                queue.push_back(value);
                Ok(())
            }
        } else {
            Err(TrySendError::Disconnected(value))
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for SyncSender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SyncSender").finish()
    }
}
