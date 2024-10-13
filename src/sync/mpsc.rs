mod receiver;
mod sender;
mod sync_sender;

use std::{collections::LinkedList, sync::Arc};

pub use receiver::*;
pub use sender::*;
pub use sync_sender::*;

use super::Mutex;

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let queue = Arc::new(Mutex::new(LinkedList::new()));

    (Sender::new(queue.clone()), Receiver::new(queue))
}

pub fn sync_channel<T>(bound: usize) -> (SyncSender<T>, Receiver<T>) {
    let queue = Arc::new(Mutex::new(LinkedList::new()));

    (SyncSender::new(queue.clone(), bound), Receiver::new(queue))
}
