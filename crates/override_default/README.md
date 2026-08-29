# override_default

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/nixonyh/override_default#license)
[![Crates.io](https://img.shields.io/crates/v/override_default.svg)](https://crates.io/crates/override_default)
[![Docs](https://docs.rs/override_default/badge.svg)](https://docs.rs/override_default/latest/override_default/)
[![CI](https://github.com/nixonyh/override_default/workflows/CI/badge.svg)](https://github.com/nixonyh/override_default/actions)
[![Discord](https://img.shields.io/discord/442334985471655946.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/Mhnyp6VYEQ)

**override_default** is a `#[derive(Default)]` where any field can start
from something other than its type's own default, said with a
`#[default(...)]` attribute on the field.

## Quick Start

```rust
use override_default::OverrideDefault;

#[derive(Default)]
struct Font {
    size: u32,
    weight: u32,
}

#[derive(OverrideDefault)]
enum Align {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(OverrideDefault)]
struct Label {
    #[default(String::from("Save"))] // a value, whole
    text: String,
    #[default(13)]
    size: u32,
    #[default(size: 24, weight: 400)] // Font::default(), fields overridden
    font: Font,
    #[default(::Right)] // a variant of the field's own type
    align: Align,
    #[default(..)] // an Option, filled with its inner default
    fallback: Option<Font>,
}

let label = Label::default();

assert_eq!(label.text, "Save");
assert_eq!(label.size, 13);
assert_eq!((label.font.size, label.font.weight), (24, 400));
assert!(matches!(label.align, Align::Right));
assert!(label.fallback.is_some());
```

A field with no `#[default]` keeps its type's own default, exactly like
the standard derive. Every type parameter has to be `Default` for the
whole to be, same as `#[derive(Default)]`.

## The `#[default(...)]` forms

| Written | Means |
| --- | --- |
| `#[default(px(4))]` | this value, whole |
| `#[default(::Bold)]` | a variant of the field's own type - no need to name the enum |
| `#[default(size: 24, weight: 400)]` | the field type's `Default`, with these of its fields overridden (`0: 1, 1: 2` for a tuple) |
| `#[default(_, 8, ..)]` | the same, written as the pattern it looks like: `_` and `..` keep what the default had |
| `#[default(..)]` | an `Option` field holding its inner type's default rather than `None` |
| `#[default({ EXPR })]` | braces force the value form, for an expression that would otherwise read as one of the above |

## Enums

An enum starts in the variant marked `#[default]`, and that variant's
own fields take `#[default(...)]` like any other:

```rust
use override_default::OverrideDefault;

#[derive(Debug, OverrideDefault, PartialEq)]
enum Edge {
    Square,
    #[default]
    Round(#[default(3)] u32),
}

assert_eq!(Edge::default(), Edge::Round(3));
```

## License

`override_default` is dual-licensed under either:

- MIT License ([LICENSE-MIT](/LICENSE-MIT) or <http://opensource.org/licenses/MIT>)
- Apache License, Version 2.0 ([LICENSE-APACHE](/LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)

at your option.
