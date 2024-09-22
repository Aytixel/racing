use std::{
    cell::OnceCell,
    collections::VecDeque,
    future::Future,
    pin::pin,
    sync::{Arc, Barrier, Condvar, Mutex},
    task::{Context, Poll, Wake},
    thread::{self, sleep},
    time::Duration,
};

use crate::{thread::spawn, BoxFuture};

thread_local! {
    pub(crate) static FUTURE_QUEUE: OnceCell<FutureQueue> = OnceCell::new();
}

#[derive(Clone)]
pub(crate) struct FutureQueue {
    queue: Arc<(Mutex<VecDeque<BoxFuture<'static, ()>>>, Condvar)>,
}

impl FutureQueue {
    fn new() -> Self {
        Self {
            queue: Arc::new((Mutex::new(VecDeque::new()), Condvar::new())),
        }
    }

    pub fn get_thread_local() -> Self {
        FUTURE_QUEUE.with(|future_queue| {
            future_queue
                .get()
                .expect("Can't get future thread queue")
                .clone()
        })
    }

    fn set_thread_local(&self) {
        FUTURE_QUEUE.with(|future_queue| {
            future_queue.set(self.clone()).ok();
        })
    }

    pub fn send(&self, future: BoxFuture<'static, ()>) {
        self.queue
            .0
            .lock()
            .expect("Thread is poisoned")
            .push_back(future);
    }

    fn get(&self) -> Option<BoxFuture<'static, ()>> {
        self.queue.0.lock().expect("Thread is poisoned").pop_front()
    }

    fn drain(&self) -> Vec<BoxFuture<'static, ()>> {
        self.queue
            .0
            .lock()
            .expect("Thread is poisoned")
            .drain(..)
            .collect()
    }

    fn wait(&self) {
        let queue = self.queue.0.lock().expect("Thread is poisoned");

        match queue.len() {
            0 => {
                self.queue.1.wait(queue).ok();
            }
            1 => sleep(Duration::from_millis(1)),
            _ => {
                self.queue.1.notify_one();

                sleep(Duration::from_millis(1));
            }
        }
    }
}

enum ThreadWaker {
    Current,
    Threaded(usize),
}

impl ThreadWaker {
    pub fn current() -> Arc<Self> {
        Arc::new(Self::Current)
    }

    pub fn threaded(worker_thread: usize) -> Arc<Self> {
        Arc::new(Self::Threaded(worker_thread))
    }
}

impl Wake for ThreadWaker {
    fn wake(self: Arc<Self>) {}
}

pub struct Runtime {
    waker: Arc<ThreadWaker>,
}

impl Runtime {
    pub fn current() -> Self {
        Self {
            waker: ThreadWaker::current(),
        }
    }

    pub fn threaded(worker_thread: usize) -> Self {
        Self {
            waker: ThreadWaker::threaded(worker_thread),
        }
    }

    pub fn block_on<T: Send + 'static>(
        &self,
        future: impl Future<Output = T> + Send + 'static,
    ) -> T {
        let queue = FutureQueue::new();

        queue.set_thread_local();

        match *self.waker {
            ThreadWaker::Current => self.block_on_current(queue, future),
            ThreadWaker::Threaded(worker_thread) => {
                self.block_on_threaded(queue, future, worker_thread)
            }
        }
    }

    fn block_on_current<T: Send + 'static>(
        &self,
        queue: FutureQueue,
        future: impl Future<Output = T> + Send + 'static,
    ) -> T {
        let mut future = pin!(future);
        let waker = self.waker.clone().into();
        let mut context = Context::from_waker(&waker);

        loop {
            if let Poll::Ready(result) = future.as_mut().poll(&mut context) {
                return result;
            }

            for mut future in queue.drain() {
                if let Poll::Pending = future.as_mut().poll(&mut context) {
                    queue.send(future);
                }
            }

            sleep(Duration::from_millis(1));
        }
    }

    fn block_on_threaded<T: Send + 'static>(
        &self,
        queue: FutureQueue,
        future: impl Future<Output = T> + Send + 'static,
        worker_thread: usize,
    ) -> T {
        if worker_thread < 1 {
            panic!("You should use at least 1 worker threads");
        }

        for _ in 0..worker_thread {
            let queue: FutureQueue = queue.clone();

            thread::spawn(move || {
                queue.set_thread_local();

                let waker = ThreadWaker::threaded(worker_thread).into();
                let mut context = Context::from_waker(&waker);

                loop {
                    if let Some(mut future) = queue.get() {
                        if let Poll::Pending = future.as_mut().poll(&mut context) {
                            queue.send(future);
                        }
                    }

                    queue.wait();
                }
            });
        }

        let result = Arc::new((Mutex::new(None), Barrier::new(2)));

        spawn({
            let result = result.clone();

            Box::pin(async move {
                *result
                    .0
                    .lock()
                    .expect("Worker thread can't lock the result") = Some(future.await);
                result.1.wait();
            })
        });

        result.1.wait();

        let mut result = result.0.lock().expect("Main thread can't lock the result");

        result.take().unwrap()
    }
}
