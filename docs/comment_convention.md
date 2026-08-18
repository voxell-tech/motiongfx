# Comment convention

## When to write one

Write a comment only where the code doesn't already say it clearly -
a non-obvious reason, a public-facing function/struct/mod others will
call without reading its body. Most code needs none.

## Keep it concise

Exactly what needs explaining, nothing else. It describes the current
code, not the diff that produced it - no "was X, now Y", no
changelog. Write it for a reader with no memory of that history: if
understanding the comment depends on knowing what changed, it isn't
self-contained yet.

## ASCII only

No em dashes, smart quotes, or other non-ASCII punctuation. Plain
hyphens (`-`), straight quotes, and the rest of ASCII cover every
case.

## What a doc comment covers

A doc comment (`///`/`//!`) describes the thing it's attached to as a
whole - what it's for, how to use it. It should read the same
regardless of who calls it or how.

Don't use it to enumerate specific callers or generic instantiations.
A generic function gets called with types its author will never see;
a list like "`T=String` does X, `T=Name` does Y" goes stale the
moment a new caller shows up, and nothing forces it to be kept in
sync.

## Where call-site behavior belongs

Behavior that only applies to one call site, one branch, or one
specific interaction with another subsystem belongs as a `//` comment
right at that code, not hoisted into the function's doc comment. If a
comment only makes sense once you're looking at the line it explains,
that's where it lives.

### Example

```rust
/// A single-line text input.
fn text_field<T: FromReflect>(...) { ... }
```

not

```rust
/// A single-line text input.
///
/// `T` is whatever the leaf actually is - `String` reads and writes
/// through as-is, `Name` converts at the edges.
fn text_field<T: FromReflect>(...) { ... }
```

and the reason one specific call listens for a particular event stays
next to that call, not in the function doc above it:

```rust
fn text_field<T: FromReflect>(...) {
    // ...
    // `TextEditChange` also fires on a bare cursor move; `write` is
    // what keeps that from writing back an unchanged value.
    ui.world.entity_mut(text_input).observe(...);
}
```
