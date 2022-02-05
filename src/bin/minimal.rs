#![allow(unused)]
#![allow(non_camel_case_types)]

use std::{marker::PhantomData, os::raw::c_int, sync::Arc, thread, time::Duration};

/// Provided by C library
struct ec_subt;

/// Wrapper provided by Rust crate
struct Sub(ec_subt);

impl Default for Sub {
    fn default() -> Sub {
        Sub(unsafe { std::mem::zeroed() })
    }
}

/// Provided by C library
struct ecx_context {
    sublist: *mut ec_subt,
    subcount: *mut c_int,
}

/// Wrapper provided by Rust crate
struct Context<'a> {
    context: ecx_context,
    _phantom: PhantomData<&'a ()>,
}

// Good or bad?
unsafe impl<'a> Send for Context<'a> {}
unsafe impl<'a> Sync for Context<'a> {}

impl<'a> Context<'a> {
    /// This is how the Rust crate currently creates a new `Context` instance. Could this be done better?
    fn new(subs: &mut [Sub], subcount: &mut c_int) -> Self {
        Self {
            context: ecx_context {
                sublist: &mut subs[0].0,
                subcount: &mut *subcount,
            },
            _phantom: PhantomData,
        }
    }

    fn send_stuff_mut(&mut self) {
        // ...
    }

    fn receive_stuff_mut(&mut self) {
        // ...
    }

    fn get_something(&self) {
        // ...
    }
}

fn main() {
    let mut subs: [Sub; 1] = Default::default();
    let mut subcount: c_int = Default::default();

    let c = Context::new(&mut subs, &mut subcount);

    let c = Arc::new(parking_lot::RwLock::new(c));

    let thread_c = c.clone();

    dbg!(c.read().get_something());

    let t = thread::spawn(move || {
        for i in 0..10 {
            {
                let mut w = thread_c.write();

                w.send_stuff_mut();

                w.receive_stuff_mut();

                dbg!(thread_c.read().get_something());
            }

            thread::sleep(Duration::from_millis(100));
        }
    });

    c.write().send_stuff_mut();
    c.write().receive_stuff_mut();

    dbg!(c.read().get_something());
}
