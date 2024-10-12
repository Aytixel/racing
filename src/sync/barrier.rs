use std::{
    cell::Cell,
    fmt,
    future::{poll_fn, Future},
    task::Poll,
};

use super::Mutex;

#[derive(Debug)]
pub struct BarrierWaitResult(bool);

impl BarrierWaitResult {
    pub fn is_leader(&self) -> bool {
        self.0
    }
}

struct BarrierState {
    counter: usize,
    generation: usize,
}

pub struct Barrier {
    n: usize,
    state: Mutex<BarrierState>,
}

impl Barrier {
    pub const fn new(n: usize) -> Barrier {
        Barrier {
            n,
            state: Mutex::new(BarrierState {
                counter: 0,
                generation: 0,
            }),
        }
    }

    pub async fn wait(&self) -> BarrierWaitResult {
        let (leader, generation) = {
            let mut state = self.state.lock().await;
            let result = (state.counter == 0, state.generation);

            state.counter += 1;

            if state.counter >= self.n {
                state.counter = 0;
                state.generation += 1;
            }

            result
        };
        let mut state = Cell::new(Box::pin(self.state.lock()));

        poll_fn(|context| {
            let Poll::Ready(state_) = state.get_mut().as_mut().poll(context) else {
                return Poll::Pending;
            };

            state.set(Box::pin(self.state.lock()));

            if state_.generation > generation {
                Poll::Ready(BarrierWaitResult(leader))
            } else {
                Poll::Pending
            }
        })
        .await
    }
}

impl fmt::Debug for Barrier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Barrier").field("number", &self.n).finish()
    }
}
