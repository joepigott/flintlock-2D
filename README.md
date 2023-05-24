# Flintlock

[![Rust](https://github.com/joepigott/flintlock-2D/actions/workflows/rust.yml/badge.svg)](https://github.com/joepigott/flintlock-2D/actions/workflows/rust.yml)

Flintlock is a 2D renderer written in Rust, using the 
[vulkano](https://github.com/vulkano-rs/vulkano) Vulkan bindings. The goal is 
to create a simple 2D renderer and level editor for creating games.

I have an irrational fear of having things I don't use, so it will only contain
essential features: renderer and rendering systems, and a level editor. This
list will likely change as I find things that would be convenient.

---

Current status:
It's possible to render an arbitrary number of primitives, but only those that implement `ColorRenderable`.
Textured primitives are being worked on.
