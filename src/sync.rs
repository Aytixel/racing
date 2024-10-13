mod barrier;
mod condvar;
mod mutex;
mod rwlock;

pub mod mpsc;

pub use barrier::*;
pub use condvar::*;
pub use mutex::*;
pub use rwlock::*;

#[derive(Debug)]
pub enum TryLock<T> {
    Guard(T),
    WouldBlock,
}
