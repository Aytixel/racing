use std::{
    cell::Cell,
    future::{poll_fn, Future},
    panic::{RefUnwindSafe, UnwindSafe},
    task::Poll,
    time::{Duration, Instant},
};

use super::{Mutex, MutexGuard};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaitTimeoutResult(bool);

impl WaitTimeoutResult {
    pub fn timed_out(&self) -> bool {
        self.0
    }
}

#[derive(Debug, Default)]
pub struct CondvarState {
    counter: usize,
    queue: Vec<usize>,
}

#[derive(Debug, Default)]
pub struct Condvar {
    state: Mutex<CondvarState>,
}

impl RefUnwindSafe for Condvar {}
impl UnwindSafe for Condvar {}

impl Condvar {
    pub const fn new() -> Condvar {
        Condvar {
            state: Mutex::new(CondvarState {
                counter: 0,
                queue: Vec::new(),
            }),
        }
    }

    pub async fn wait<'a, T>(&self, guard: MutexGuard<'a, T>) -> MutexGuard<'a, T> {
        poll_wait!(
            self.state,
            guard,
            |state: MutexGuard<'_, CondvarState>, _guard, id| {
                if !state.queue.contains(&id) {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            }
        )
    }

    pub async fn wait_while<'a, T, F>(
        &self,
        guard: MutexGuard<'a, T>,
        mut condition: F,
    ) -> MutexGuard<'a, T>
    where
        F: FnMut(&mut T) -> bool,
    {
        poll_wait!(
            self.state,
            guard,
            |state: MutexGuard<'_, CondvarState>, guard, id| {
                if !state.queue.contains(&id) && condition(guard) {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            }
        )
    }

    pub async fn wait_timeout<'a, T>(
        &self,
        guard: MutexGuard<'a, T>,
        dur: Duration,
    ) -> (MutexGuard<'a, T>, WaitTimeoutResult) {
        poll_wait!(
            self.state,
            guard,
            dur,
            |state: MutexGuard<'_, CondvarState>, _guard, id| {
                if !state.queue.contains(&id) {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            }
        )
    }

    pub async fn wait_timeout_while<'a, T, F>(
        &self,
        guard: MutexGuard<'a, T>,
        dur: Duration,
        mut condition: F,
    ) -> (MutexGuard<'a, T>, WaitTimeoutResult)
    where
        F: FnMut(&mut T) -> bool,
    {
        poll_wait!(
            self.state,
            guard,
            dur,
            |state: MutexGuard<'_, CondvarState>, guard, id| {
                if !state.queue.contains(&id) && condition(guard) {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            }
        )
    }

    pub async fn notify_one(&self) {
        self.state.lock().await.queue.pop();
    }

    pub async fn notify_all(&self) {
        self.state.lock().await.queue.drain(..);
    }
}

macro_rules! poll_wait {
    ($state:expr, $guard:expr, $timeout:expr, $closure:expr) => {{
        let id = {
            let mut state = $state.lock().await;
            let id = state.counter;

            state.queue.push(id);
            state.counter += 1;

            id
        };
        let mutex = $guard.unlock();
        let mut state = Cell::new(Box::pin($state.lock()));
        let mut guard = Cell::new(Box::pin(mutex.lock()));

        let instant = Instant::now();
        let mut has_timed_out = false;

        poll_fn(|context| {
            if instant.elapsed() >= $timeout {
                has_timed_out = true;
                return Poll::Ready(());
            }

            let Poll::Ready(state_) = state.get_mut().as_mut().poll(context) else {
                return Poll::Pending;
            };

            state.set(Box::pin($state.lock()));

            let Poll::Ready(mut guard_) = guard.get_mut().as_mut().poll(context) else {
                return Poll::Pending;
            };

            let poll_result = $closure(state_, &mut guard_, id);

            guard.set(Box::pin(guard_.unlock().lock()));

            poll_result
        })
        .await;

        $state.lock().await.queue.retain(|id_| id_ == &id);
        (mutex.lock().await, WaitTimeoutResult(has_timed_out))
    }};

    ($state:expr, $guard:expr, $closure:expr) => {{
        let id = {
            let mut state = $state.lock().await;
            let id = state.counter;

            state.queue.push(id);
            state.counter += 1;

            id
        };
        let mutex = $guard.unlock();
        let mut state = Cell::new(Box::pin($state.lock()));
        let mut guard = Cell::new(Box::pin(mutex.lock()));

        poll_fn(|context| {
            let Poll::Ready(state_) = state.get_mut().as_mut().poll(context) else {
                return Poll::Pending;
            };

            state.set(Box::pin($state.lock()));

            let Poll::Ready(mut guard_) = guard.get_mut().as_mut().poll(context) else {
                return Poll::Pending;
            };

            let poll_result = $closure(state_, &mut guard_, id);

            guard.set(Box::pin(guard_.unlock().lock()));

            poll_result
        })
        .await;

        $state.lock().await.queue.retain(|id_| id_ == &id);
        mutex.lock().await
    }};
}

use poll_wait;
