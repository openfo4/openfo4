use event_listener::{Event, EventListener};
use parking_lot::{Mutex, ReentrantMutex};

#[derive(Debug, Default)]
pub(crate) struct ObjectLock {
    lock: ReentrantMutex<()>,
    event: Mutex<Event>,
}

impl ObjectLock {
    pub(crate) fn new() -> Self {
        Self {
            lock: ReentrantMutex::new(()),
            event: Mutex::new(Event::new()),
        }
    }

    /// Attempts to lock the `ObjectLock`.
    ///
    /// On success `Ok` is returned and the lock is acquired for the current thread. It must be
    /// released with [`unlock`] on the same thread.
    /// On failure `Err` is returned with an [`EventListener`] that will become ready when the lock
    /// can be polled again.
    ///
    /// The lock is reentrant.
    pub(crate) fn lock(&self) -> Result<(), EventListener> {
        if let Some(guard) = self.lock.try_lock() {
            core::mem::forget(guard);
            return Ok(());
        };

        let event = self.event.lock();
        let listener = event.listen();

        if let Some(guard) = self.lock.try_lock() {
            core::mem::forget(guard);
            return Ok(());
        };

        Err(listener)
    }

    pub fn try_lock(&self) -> bool {
        if let Some(guard) = self.lock.try_lock() {
            core::mem::forget(guard);
            true
        } else {
            false
        }
    }

    /// Unlocks the `ObjectLock` on the current thread.
    ///
    /// # Safety
    ///
    /// - The lock must have been acquired for this `ObjectLock` before calling `unlock`.
    /// - The lock must have been acquired on the same thread.
    pub(crate) unsafe fn unlock(&self) {
        unsafe {
            self.lock.force_unlock();
        }

        // Notify all listeners
        // The listener count cannot change because we have acquired the
        // exclusive lock here.
        let event = self.event.lock();
        event.notify_relaxed(event.total_listeners());
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Barrier;

    use futures_lite::future::block_on;

    use crate::runtime::lock::ObjectLock;

    fn lock_blocking(lock: &ObjectLock) {
        loop {
            match lock.lock() {
                Ok(()) => break,
                Err(listener) => block_on(listener),
            }
        }
    }

    #[test]
    fn object_lock_lock_reentrant() {
        let lock = ObjectLock::new();
        lock.lock().unwrap();
        lock.lock().unwrap();

        unsafe {
            lock.unlock();
            lock.unlock();
        }
    }

    #[test]
    fn object_lock_and_unlock() {
        let lock = ObjectLock::new();

        std::thread::scope(|s| {
            for _ in 0..3 {
                s.spawn(|| {
                    lock_blocking(&lock);

                    unsafe {
                        lock.unlock();
                    }
                });
            }
        })
    }

    #[test]
    fn object_lock_drop_lock() {
        let lock = ObjectLock::new();

        std::thread::scope(|s| {
            for _ in 0..2 {
                s.spawn(|| {
                    lock_blocking(&lock);

                    unsafe {
                        lock.unlock();
                    }
                });
            }

            s.spawn(|| {
                if let Ok(()) = lock.lock() {
                    unsafe {
                        lock.unlock();
                    }
                }
            });
        })
    }

    #[test]
    fn object_lock_drop_notify_ready() {
        let lock = ObjectLock::new();

        let listeners_acquired = Barrier::new(3);

        std::thread::scope(|s| {
            lock.lock().unwrap();

            s.spawn(|| {
                let listen_a = lock.lock().unwrap_err();
                listeners_acquired.wait();
                block_on(listen_a);
            });

            s.spawn(|| {
                let listen_b = lock.lock().unwrap_err();
                listeners_acquired.wait();
                block_on(listen_b);
                lock.lock().unwrap();
            });

            listeners_acquired.wait();

            unsafe {
                lock.unlock();
            }
        });
    }
}
