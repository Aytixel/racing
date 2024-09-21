use std::{
    cell::OnceCell,
    collections::VecDeque,
    future::Future,
    pin::pin,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
    task::{Context, Poll, Wake},
    thread::{self, sleep},
    time::Duration,
};

use crate::{thread::spawn, BoxFuture};

thread_local! {
    pub(crate) static FUTURE_SENDER: OnceCell<Sender<BoxFuture<'static, ()>>> = OnceCell::new();
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
        let (sender, receiver) = channel::<BoxFuture<'static, ()>>();

        FUTURE_SENDER.with(|sender_| {
            sender_
                .set(sender.clone())
                .expect("Can't block on a thread you already block on");
        });

        match *self.waker {
            ThreadWaker::Current => self.block_on_current(sender, receiver, future),
            ThreadWaker::Threaded(worker_thread) => {
                self.block_on_threaded(sender, receiver, future, worker_thread)
            }
        }
    }

    fn block_on_current<T: Send + 'static>(
        &self,
        sender: Sender<BoxFuture<'static, ()>>,
        receiver: Receiver<BoxFuture<'static, ()>>,
        future: impl Future<Output = T> + Send + 'static,
    ) -> T {
        let mut future = pin!(future);
        let waker = self.waker.clone().into();
        let mut context = Context::from_waker(&waker);

        loop {
            match future.as_mut().poll(&mut context) {
                Poll::Ready(result) => {
                    return result;
                }
                Poll::Pending => sleep(Duration::from_millis(1)),
            }

            let mut queue: Vec<BoxFuture<'static, ()>> = Vec::new();

            while let Ok(mut future) = receiver.try_recv() {
                if let Poll::Pending = future.as_mut().poll(&mut context) {
                    queue.push(future);
                }
            }

            queue.into_iter().for_each(|future| {
                sender.send(future).ok();
            });
        }
    }

    fn block_on_threaded<T: Send + 'static>(
        &self,
        sender: Sender<BoxFuture<'static, ()>>,
        receiver: Receiver<BoxFuture<'static, ()>>,
        future: impl Future<Output = T> + Send + 'static,
        worker_thread: usize,
    ) -> T {
        let waker = self.waker.clone().into();
        let mut context = Context::from_waker(&waker);
        let mut handle = pin!(spawn(Box::pin(future)));
        let future_queue: Arc<Mutex<VecDeque<BoxFuture<'static, ()>>>> =
            Arc::new(Mutex::new(VecDeque::new()));

        for _ in 0..worker_thread {
            let sender = sender.clone();
            let future_queue = future_queue.clone();

            thread::spawn(move || {
                FUTURE_SENDER.with(|sender_| {
                    sender_.set(sender).ok();
                });

                let waker = ThreadWaker::threaded(worker_thread).into();
                let mut context = Context::from_waker(&waker);

                loop {
                    if let Some(mut future) = {
                        let mut future_queue =
                            future_queue.lock().expect("Worker thread is poisoned");

                        future_queue.pop_front()
                    } {
                        if let Poll::Pending = future.as_mut().poll(&mut context) {
                            let mut future_queue =
                                future_queue.lock().expect("Worker thread is poisoned");

                            future_queue.push_back(future);
                        }
                    }

                    sleep(Duration::from_millis(1));
                }
            });
        }

        loop {
            if let Poll::Ready(result) = handle.as_mut().poll(&mut context) {
                return result;
            }

            while let Ok(future) = receiver.try_recv() {
                let mut future_queue = future_queue.lock().expect("Main thread is poisoned");

                future_queue.push_back(future);
            }

            sleep(Duration::from_millis(1));
        }
    }
}
