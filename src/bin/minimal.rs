use std::{marker::PhantomData, os::raw::c_int, sync::Arc, thread, time::Duration};

struct ec_slavet;

struct Slave(ec_slavet);

impl Drop for Slave {
    fn drop(&mut self) {
        println!("Drop slave");
    }
}

impl Default for Slave {
    fn default() -> Slave {
        Slave(unsafe { std::mem::zeroed() })
    }
}

struct ecx_context {
    slavelist: *mut ec_slavet,
    slavecount: *mut c_int,
}

struct Context<'a, const SLAVE_COUNT: usize> {
    context: ecx_context,
    slaves: [Slave; SLAVE_COUNT],
    slavecount: i32,
    counter: usize,
    _phantom: PhantomData<&'a ()>,
}

unsafe impl<'a, const SLAVE_COUNT: usize> Send for Context<'a, SLAVE_COUNT> {}
unsafe impl<'a, const SLAVE_COUNT: usize> Sync for Context<'a, SLAVE_COUNT> {}

impl<'a, const SLAVE_COUNT: usize> Context<'a, SLAVE_COUNT> {
    fn new(mut slaves: [Slave; SLAVE_COUNT], mut slavecount: c_int) -> Self {
        Self {
            context: ecx_context {
                slavelist: &mut slaves[0].0,
                slavecount: &mut slavecount,
            },
            // These must be here otherwise anything passed into ecx_context is dropped immediately
            slaves,
            slavecount,
            counter: 0,
            _phantom: PhantomData,
        }
    }

    fn do_mut_thing(&mut self) {
        self.counter += 1;
    }

    fn do_ref_thing(&self) {
        println!("Count {}", self.counter);
    }
}

fn main() {
    let slaves: [Slave; 1] = Default::default();
    let slavecount: c_int = Default::default();

    let mut c = Context::new(slaves, slavecount);

    let mut c = Arc::pin(parking_lot::RwLock::new(c));

    let thread_c = c.clone();

    let t = thread::spawn(move || {
        for i in 0..10 {
            {
                let mut w = thread_c.write();

                w.do_mut_thing();

                // w.do_ref_thing();
            }

            thread::sleep(Duration::from_millis(100));
        }
    });

    c.write().do_mut_thing();
    c.read().do_ref_thing();

    t.join();

    println!("After thread");

    c.read().do_ref_thing();
}
