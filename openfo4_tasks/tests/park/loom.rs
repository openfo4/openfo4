#![cfg(loom)]

use loom::sync::Arc;
use loom::thread;
use openfo4_tasks::park::Parker;

#[test]
fn smoke() {
    loom::model(|| {
        let parker = Arc::new(Parker::new());
        let unparker = parker.clone();

        thread::spawn(move || {
            parker.park();
        });

        unparker.unpark();
    });
}
