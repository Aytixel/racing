use std::{
    cell::UnsafeCell,
    fmt,
    future::poll_fn,
    ops::{Deref, DerefMut},
    panic::{RefUnwindSafe, UnwindSafe},
    sync::atomic::{AtomicUsize, Ordering},
    task::Poll,
};

use super::TryLock;

pub struct RwLockReadGuard<'a, T> {
    rwlock: &'a RwLock<T>,
}

unsafe impl<T: Sync> Sync for RwLockReadGuard<'_, T> {}

impl<T> Drop for RwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        self.rwlock.locked.fetch_sub(1, Ordering::Relaxed);
    }
}

impl<T> Deref for RwLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            &*self
                .rwlock
                .value
                .as_ref()
                .expect("RwLock value dropped before deref")
                .get()
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for RwLockReadGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RwLockReadGuard")
            .field("value", &self.rwlock.value)
            .finish()
    }
}

impl<T: fmt::Display> fmt::Display for RwLockReadGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            &*self
                .rwlock
                .value
                .as_ref()
                .expect("RwLock value dropped before fmt")
                .get()
        }
        .fmt(f)
    }
}

pub struct RwLockWriteGuard<'a, T> {
    rwlock: &'a RwLock<T>,
}

unsafe impl<T: Sync> Sync for RwLockWriteGuard<'_, T> {}

impl<T> Drop for RwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        self.rwlock.locked.store(1, Ordering::Relaxed);
    }
}

impl<T> Deref for RwLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            &*self
                .rwlock
                .value
                .as_ref()
                .expect("RwLock value dropped before deref")
                .get()
        }
    }
}

impl<T> DerefMut for RwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            &mut *self
                .rwlock
                .value
                .as_ref()
                .expect("RwLock value dropped before deref")
                .get()
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for RwLockWriteGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RwLockWriteGuard")
            .field("value", &self.rwlock.value)
            .finish()
    }
}

impl<T: fmt::Display> fmt::Display for RwLockWriteGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            &*self
                .rwlock
                .value
                .as_ref()
                .expect("RwLock value dropped before fmt")
                .get()
        }
        .fmt(f)
    }
}

#[derive(Default)]
pub struct RwLock<T> {
    locked: AtomicUsize,
    value: Option<UnsafeCell<T>>,
}

impl<T> RefUnwindSafe for RwLock<T> {}
unsafe impl<T: Send> Send for RwLock<T> {}
unsafe impl<T: Send + Sync> Sync for RwLock<T> {}
impl<T> UnwindSafe for RwLock<T> {}

impl<T> RwLock<T> {
    pub const fn new(t: T) -> RwLock<T> {
        RwLock {
            locked: AtomicUsize::new(1),
            value: Some(UnsafeCell::new(t)),
        }
    }

    pub async fn read(&self) -> RwLockReadGuard<'_, T> {
        poll_fn(|_context| {
            if let Ok(_) = self
                .locked
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |locked| {
                    (locked > 0).then_some(locked + 1)
                })
            {
                Poll::Ready(RwLockReadGuard { rwlock: &self })
            } else {
                Poll::Pending
            }
        })
        .await
    }

    pub fn try_read(&self) -> TryLock<RwLockReadGuard<'_, T>> {
        if let Ok(_) = self
            .locked
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |locked| {
                (locked > 0).then_some(locked + 1)
            })
        {
            TryLock::Guard(RwLockReadGuard { rwlock: &self })
        } else {
            TryLock::WouldBlock
        }
    }

    pub async fn write(&self) -> RwLockWriteGuard<'_, T> {
        poll_fn(|_context| {
            if let Ok(_) = self
                .locked
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |locked| {
                    (locked == 1).then_some(0)
                })
            {
                Poll::Ready(RwLockWriteGuard { rwlock: &self })
            } else {
                Poll::Pending
            }
        })
        .await
    }

    pub fn try_write(&self) -> TryLock<RwLockWriteGuard<'_, T>> {
        if let Ok(_) = self
            .locked
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |locked| {
                (locked == 1).then_some(0)
            })
        {
            TryLock::Guard(RwLockWriteGuard { rwlock: &self })
        } else {
            TryLock::WouldBlock
        }
    }

    pub fn into_inner(mut self) -> T
    where
        T: Sized,
    {
        self.value
            .take()
            .expect("RwLock value dropped before into_inner")
            .into_inner()
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.value
            .as_mut()
            .expect("RwLock value dropped before get_mut")
            .get_mut()
    }
}

impl<T: fmt::Debug> fmt::Debug for RwLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RwLock")
            .field("value", &self.value)
            .finish()
    }
}

impl<T> From<T> for RwLock<T> {
    fn from(value: T) -> Self {
        RwLock::new(value)
    }
}
