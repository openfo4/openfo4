use std::task::{Context, Poll, Waker};

use futures_lite::FutureExt;
use futures_util::stream::{FuturesUnordered, StreamExt};

use crate::Task;

#[derive(Debug)]
pub struct TaskQueue<T> {
    tasks: FuturesUnordered<Task<T>>,
}

impl<T> TaskQueue<T> {
    pub fn new() -> Self {
        Self {
            tasks: FuturesUnordered::new(),
        }
    }

    pub fn push(&mut self, task: Task<T>) {
        self.tasks.push(task);
    }

    pub fn pop(&mut self) -> Option<T> {
        let mut cx = Context::from_waker(Waker::noop());

        match self.tasks.next().poll(&mut cx) {
            Poll::Ready(Some(v)) => Some(v),
            Poll::Ready(None) => None,
            Poll::Pending => None,
        }
    }
}

impl<T> Default for TaskQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}
