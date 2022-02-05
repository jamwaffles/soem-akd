I'm attempting to use a Rust-wrapped C library ([SOEM-rs](https://github.com/matwey/SOEM-rs) if you're interested). This library provides a `Context` type that pretty much behaves as a global variable, taking many arguments by reference, as this is what the C library uses behind the scenes:

```rust
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

impl<'a> Context<'a> {
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
```

I need to use `Context` in multiple threads; one which reads/writes data through a network interface and the other (main thread) with some business logic in it. Both threads need to call into the C functions which all read and write to `ecx_context`.

I added the following to get around the error printed by the compiler:

```rust
unsafe impl<'a> Send for Context<'a> {}
unsafe impl<'a> Sync for Context<'a> {}
```
The C library claims `ecx_context` and the rest of itself is threadsafe, and the examples included with it do spawn threads and talk to a global instance of `ecx_context`, so I'm inclined to believe it. There are also uses of mutexes in the C codebase to help its case.

**My first question:** What problems do I cause myself by implementing `Send` and/or `Sync` for `Context` (ne `ecx_context`), if we assume `ecx_context` is threadsafe?

---

As for the actual threading, here's an example using `parking_lot`:

```rust
fn main() {
    let mut subs: [Sub; 1] = Default::default();
    let mut subcount: c_int = Default::default();

    let c = Context::new(&mut subs, &mut subcount);
    let c = Arc::new(parking_lot::RwLock::new(c));

    let thread_c = c.clone();

    dbg!(c.read().get_something());

    let t = thread::spawn(move || {
        loop {
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
```

This works (modulo any issues I don't know about with `Sync`/`Send`), however:

**Question 2:** if the C library really is threadsafe, is there a way to safely share `Context` between threads without using some kind of lock on the Rust side? I feel the "double-lock" is unnecessary but I could be wrong. I ask because the C code does it; supposedly-thread-safe-ly calling functions that modify the global `ecx_context` from multiple threads.

I'd also like to clean up the code somewhat; in the real code there are _a lot_ of calls to methods on `Context` so it would be nice to reduce all the `read()`s and `write()`s from `parking_lot`.

Perhaps I'm barking up the wrong tree and there's a completely different, better way to model this entire thing in Rust?

And finally **Question 3:** Would `Pin` help anything here? I've read a few bits about it but I'm not clear if it would be useful in my case.

Full example code [on the playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=ce40861ecd5f1be7c2e122070d23d499).
