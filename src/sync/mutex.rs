use std::{
    cell::UnsafeCell,
    fmt,
    future::poll_fn,
    ops::{Deref, DerefMut},
    panic::{RefUnwindSafe, UnwindSafe},
    sync::atomic::{AtomicBool, Ordering},
    task::Poll,
};

use super::TryLock;

pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
}

unsafe impl<T: Sync> Sync for MutexGuard<'_, T> {}

impl<'a, T> MutexGuard<'a, T> {
    pub(crate) fn unlock(self) -> &'a Mutex<T> {
        self.mutex.locked.store(false, Ordering::Relaxed);
        self.mutex
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.locked.store(false, Ordering::Relaxed);
    }
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            &*self
                .mutex
                .value
                .as_ref()
                .expect("Mutex value dropped before deref")
                .get()
        }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            &mut *self
                .mutex
                .value
                .as_ref()
                .expect("Mutex value dropped before deref")
                .get()
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MutexGuard")
            .field("value", &self.mutex.value)
            .finish()
    }
}

impl<T: fmt::Display> fmt::Display for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            &*self
                .mutex
                .value
                .as_ref()
                .expect("Mutex value dropped before fmt")
                .get()
        }
        .fmt(f)
    }
}

#[derive(Default)]
pub struct Mutex<T> {
    locked: AtomicBool,
    value: Option<UnsafeCell<T>>,
}

impl<T> RefUnwindSafe for Mutex<T> {}
unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}
impl<T> UnwindSafe for Mutex<T> {}

impl<T> Mutex<T> {
    pub const fn new(t: T) -> Mutex<T> {
        Mutex {
            locked: AtomicBool::new(false),
            value: Some(UnsafeCell::new(t)),
        }
    }

    pub async fn lock(&self) -> MutexGuard<'_, T> {
        poll_fn(|_context| {
            if self.locked.fetch_and(true, Ordering::SeqCst) {
                Poll::Pending
            } else {
                Poll::Ready(MutexGuard { mutex: &self })
            }
        })
        .await
    }

    pub fn try_lock(&self) -> TryLock<MutexGuard<'_, T>> {
        if self.locked.fetch_and(true, Ordering::SeqCst) {
            TryLock::WouldBlock
        } else {
            TryLock::Guard(MutexGuard { mutex: &self })
        }
    }

    pub fn into_inner(mut self) -> T
    where
        T: Sized,
    {
        self.value
            .take()
            .expect("Mutex value dropped before into_inner")
            .into_inner()
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.value
            .as_mut()
            .expect("Mutex value dropped before get_mut")
            .get_mut()
    }
}

impl<T: fmt::Debug> fmt::Debug for Mutex<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Mutex").field("value", &self.value).finish()
    }
}

impl<T> From<T> for Mutex<T> {
    fn from(value: T) -> Self {
        Mutex::new(value)
    }
}
