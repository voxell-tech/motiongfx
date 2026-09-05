# Comment convention

## As less as possible

No comment if the code already explain itself.
When writing one, be as concise and precise if possible, no paragraphs.
Public facing APIs needs doc comments, but only to describe exactly what it is,
do not talk about any abstract stuff.

When it's a toss-up whether a comment earns its place, remove it.

A simple type's doc comment names what it is, linking the real type
it wraps or stands in for, and stops there. It does not explain why,
even when the reason is true and non-obvious - that belongs inline,
at the code that actually depends on it, not in the type's own doc.

Not:
```rust
/// One resource.
///
/// Bevy parks each resource on an entity of its own, so once that
/// entity is in hand this is a [`ComponentInspector`] and nothing
/// below here knows the difference.
pub struct ResourceInspector { ... }
```
Just:
```rust
/// Inspector for a [`Resource`].
pub struct ResourceInspector { ... }
```

## If the LSP already shows it, skip it

Hover, go-to-definition, and autocomplete already surface a signature,
a return type, a trait bound, a visibility modifier. A comment that
just restates one of those ("this returns an `H::Node`", "opaque
outside this crate, since it's `pub(crate)`") is not explaining
anything the reader can't already see one keystroke away. Write the
comment for what tooling cannot show: intent, a non-obvious
constraint, a reason.

## Keep it concise

Exactly what needs explaining, nothing else. It describes the current
code, not the diff that produced it - no "was X, now Y", no
changelog. Write it for a reader with no memory of that history: if
understanding the comment depends on knowing what changed, it isn't
self-contained yet.

## Plain text

Use plain English whenever possible. Use punctuations like :;- sparingly.

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

## Don't enumerate usages

Don't list the specific places a thing is set, read, or called from
("set at `new`, changed through `theme_mut`"). That list is almost
never exhaustive, nothing forces it to stay in sync, and it goes
stale the moment another call site shows up. Describe what the thing
is instead, and let readers find its callers themselves.

A comment should be lightweight and cheap to keep correct. Prefer the
version that stays true even after nearby code changes over the one
that is more complete today.

## State it one way

Say what a thing is, not what it isn't. Skip the contrastive half:

Not: "This is xyz instead of abc."
Just: "This is xyz."

The exception is when the contrast itself is the whole point of the
comment (a deliberate departure from what a reader would otherwise
assume), and even then the negative side stays short.

## Open with a noun phrase, not a free relative

No "what X does/wants" openers. Head the sentence with the thing
itself.

## Where call-site behavior belongs

Behavior that only applies to one call site, one branch, or one
specific interaction with another subsystem belongs as a `//` comment
right at that code, not hoisted into the function's doc comment. If a
comment only makes sense once you're looking at the line it explains,
that's where it lives.
