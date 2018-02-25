# Swear

This is intended to be a bad implementation in Rust of some approximation of
[JS A+ Promises](https://promisesaplus.com/).

This is not implemented precisely to spec, modifications have been made in an
attempt to make the API slightly more Rust-idiomatic, or altered for other
opinionated reasons.

It is only intended to be used in single-threaded applications (none of the
types implement Sync), and code is written with a bias for simplicity over
performance.

You probably want to take at [Futures](https://docs.rs/futures/0.1.18/futures/)
for a more widely supported implementation Promises / Futures / Deferreds in Rust.

## Motivation

I was playing around with [stdweb](https://github.com/koute/stdweb) and
wasm32-unknown-unknown, and discovered that out of the box, I was unable to use
futures::unsync utilities out of the box. As an exercise in exploring lifetime
management in Rust, and to create a utility I could use with stdweb, I created Swear.

The design is largely catered toward creating a utility that will be useful in
writing wasm-unknown-unknown frontends in rust, following Promise patterns used
in JS.

## Concepts

### Swear

The core utility in this package is a `Swear`, akin to a javascript Promise. A
`Swear` can have `.then` called on it exactly once, with a FnOnce callback that
is called once the `Swear` is fulfilled.

When you construct a `Swear` from scratch with `make_swear`, you will also
receive a `Completer`, an object that you call `complete` (again exactly once)
in order to fulfill the swear.

### Scheduler

There is some question as to from which stack callbacks should be called from
once a swear is fulfilled.

One option is always calling it from whichever stack is active as soon as the swear is fulfilled

Note that this may behave subtly differently in different situations:

Option A:

- Swear is created.
- Swear has then called on it with callback.
- Swear is fulfilled. <-- swear callback is called from this stack

Option B:

- Swear is created.
- Swear is fulfilled.
- Swear has then called on it with callback.  <-- swear callback is called from this stack

Wanting to avoid confusion about when the callback would be called (and
subsequently lifetimes of all variables captured by the closure), this
implementation opts to instead always "schedule" the callback for running from
some outside scheduler.

An equivalent analysis:

Option A:

- Swear is created.
- Swear has then called on it with callback.
- Swear is fulfilled. <-- swear callback is scheduled from this stack
- Once current callback/task has completed, swear callback is called from the scheduler directly.

Option B:

- Swear is created.
- Swear is fulfilled.
- Swear has then called on it with callback.  <-- swear callback is scheduled from this stack
- Once current callback/task has completed, swear callback is called from the scheduler directly.

This might be worse for cache-consistency, but unifies the 

Provided in the crate is an implementation of a simple "runqueue", a
similar JS implementation would be "window.setTimeout(callback, 0)".

This requires that the lifetime of closure (and it's references) usually has to
outlive the current stack (in fact, objects that are referenced must be alive
for the entire lifetime of the runqueue, not knowing when it will execute the
scheduled callback).

## Examples

The `tests/` directory has two simple examples of how to do simple thenning on Swears.
`and_then` is used if the callback returns another Swear, `then` is used if the
callback returns a non-swear value.
