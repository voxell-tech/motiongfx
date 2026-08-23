# Code convention

## Confirm before reaching for memory-management types

`Arc`, `Rc`, `Box<dyn Trait>`, `Mutex`, `RwLock`, `Cell`, `RefCell`, or
any other memory-management/interior-mutability type: stop and confirm
with the user before introducing one. They carry a real runtime cost -
an allocation, a reference count, a lock - and are easy to reach for
out of habit where plain ownership or borrowing would have worked;
overused, they add up and slow the program down. Explain what plain
ownership or borrowing was tried first and why it didn't work, and let
the user decide rather than reaching for one of these as the default
fix.

## Turbofish, not an annotated binding

When a generic call's type needs pinning down, prefer turbofish on
the call itself (`.collect::<Vec<_>>()`, `.parse::<i32>()`, and so on)
over annotating the binding's type to steer inference.
