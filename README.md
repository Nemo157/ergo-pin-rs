# ergo-pin

<p align="center"><i>Immobilis ergo pin</i></p>

**Ergo**nomic stack **pin**ning for Rust.

`ergo-pin` exports a single proc-macro-attribute `#[ergo_pin]` that can be applied to a
function/block/`tt`-accepting-macro-invocation to provide the "magical" `pin!`
within the scope. This `pin!` macro is equivalent to a magical function `fn
pin!<T>(t: T) -> Pin<&mut T>`, it will take in any value and return a `Pin<&mut
_>` of the value.

## Internals

Internally the `pin!` macro works by injecting extra statements before the
statement that contains it to create this `Pin`ned reference, it's probably
easiest to see what it's doing by example. Given some code like

```rust
#[ergo_pin] {
    quux(pin!(Foo::new().bar()).baz());
}
```

this will be re-written to

```rust
{
    let mut __ergo_pin_0 = Foo::new().bar();
    let __ergo_pin_0 = unsafe {
        ::core::pin::Pin::new_unchecked(&mut __ergo_pin_0)
    };
    quux(__ergo_pin_0.baz());
}
```

The expression passed to `pin!` is first bound to a local variable, this is then
shadowed by a `Pin`ned reference to itself (you may recognise this from the
`pin-utils::pin_mut!` stack pinning macro), finally the `pin!` call is replaced
with the local variable used.

Since this requires re-writing code outside the macro this can't be implemented
by as a normal `pin!` macro, which is why the main entrypoint is the
`#[ergo_pin]` attribute.
