# MotionGfx

**MotionGfx** is a backend-agnostic motion graphics framework
built on top of the [Bevy] ECS. It provides a modular foundation
for procedural animations.

## Key Features

- **Backend agnostic**: Works with any rendering backend.
- **Procedural**: Write animations with code - loops, functions, logic.
- **Type-erased**: Powered by
  [Field Path](https://github.com/voxell-tech/field_path), allowing
  runtime-flexible animation of arbitrary data.
- **Two-way playback**: Play animations both forward and backward with
  no extra computation.

## License

`motiongfx` is dual-licensed under either:

- MIT License ([LICENSE-MIT](LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

This means you can select the license you prefer!
This dual-licensing approach is the de-facto standard in the Rust ecosystem and there are [very good reasons](https://github.com/bevyengine/bevy/issues/2373) to include both.
