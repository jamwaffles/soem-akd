I'm attempting to use a Rust-wrapped C library ([SOEM-rs](https://github.com/matwey/SOEM-rs) if you're interested). This library provides a `Context` type that pretty much behaves as a global variable, taking many arguments by reference, as this is what the C library uses behind the scenes:

```rust
struct ec_subt;

struct Sub(ec_subt);

impl Default for Sub {
    fn default() -> Sub {
        Sub(unsafe { std::mem::zeroed() })
    }
}

struct ecx_context {
    sublist: *mut ec_subt,
    subcount: *mut c_int,
}

struct Context<'a> {
    context: ecx_context,
    _phantom: PhantomData<&'a ()>,
}
```

I need to use `Context` in multiple threads; one which reads/writes data through a network interface and the other (main thread) with some business logic in it. Both threads need to call into the C functions which all read and write to `ecx_context`.

My first issue is of course the `*mut _` cannot be sent between threads safely, solved/kludged with:

```rust
unsafe impl<'a> Send for Context<'a> {}
unsafe impl<'a> Sync for Context<'a> {}
```

**Question 1:** The C library _claims_ `ecx_context` and the rest of itself is threadsafe, and the examples included with it do spawn threads and talk to a global instance of `ecx_context`, so I'm inclined to believe it. **Therefore, is it safe to impl `Send` and/or `Sync` for `Context`?**

Now of course I can't share `Context` between threads so I wrap it:

```rust
let mut subs: [Sub; 1] = Default::default();
let mut subcount: c_int = Default::default();

let c = Context::new(&mut subs, &mut subcount);

let c = Arc::new(parking_lot::RwLock::new(c));
```

which is fine until I try to use it in a thread:

```rust
let thread_c = c.clone();

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
```
