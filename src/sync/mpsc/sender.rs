use std::{
    collections::LinkedList,
    fmt,
    sync::{mpsc::SendError, Arc},
};

use crate::sync::Mutex;

#[derive(Clone)]
pub struct Sender<T> {
    queue: Arc<Mutex<LinkedList<T>>>,
    sender: Arc<()>,
}

unsafe impl<T: Send> Send for Sender<T> {}
unsafe impl<T: Send> Sync for Sender<T> {}

impl<T> Sender<T> {
    pub(super) fn new(queue: Arc<Mutex<LinkedList<T>>>) -> Sender<T> {
        Sender {
            queue,
            sender: Arc::new(()),
        }
    }

    pub async fn send(&self, value: T) -> Result<(), SendError<T>> {
        if Arc::strong_count(&self.queue) - Arc::strong_count(&self.sender) == 1 {
            self.queue.lock().await.push_back(value);
            Ok(())
        } else {
            Err(SendError(value))
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for Sender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sender").finish()
    }
}
