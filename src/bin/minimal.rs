use std::{marker::PhantomData, os::raw::c_int, sync::Arc, thread, time::Duration};

struct ec_slavet;

struct Slave(ec_slavet);

impl Default for Slave {
    fn default() -> Slave {
        Slave(unsafe { std::mem::zeroed() })
    }
}

struct ecx_context {
    slavelist: *mut ec_slavet,
    slavecount: *mut c_int,
}

struct Context<'a> {
    context: ecx_context,
    _phantom: PhantomData<&'a ()>,
}

unsafe impl<'a> Send for Context<'a> {}
unsafe impl<'a> Sync for Context<'a> {}

impl<'a> Context<'a> {
    fn new(slaves: &'a mut [Slave], slavecount: &'a mut c_int) -> Self {
        Self {
            context: ecx_context {
                slavelist: &mut slaves[0].0,
                slavecount: &mut *slavecount,
            },
            _phantom: PhantomData,
        }
    }

    fn do_mut_thing(&mut self) {
        // ...
    }

    fn do_ref_thing(&self) {
        // ...
    }
}

fn main() {
    let mut slaves: [Slave; 1] = Default::default();
    let mut slavecount: c_int = Default::default();

    let mut c = Context::new(&mut slaves, &mut slavecount);

    let mut c = Arc::pin(parking_lot::RwLock::new(c));

    let thread_c = c.clone();

    thread::spawn(move || loop {
        {
            let mut w = thread_c.write();

            w.do_mut_thing();
        }

        thread::sleep(Duration::from_millis(100));
    });

    c.write().do_mut_thing();
    c.read().do_ref_thing();
}
