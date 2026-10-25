# 🦎 Newt Hype

[![Crates.io](https://img.shields.io/crates/v/newt-hype)](https://crates.io/crates/newt-hype)
[![docs.rs](https://img.shields.io/docsrs/newt-hype?label=docs)](https://docs.rs/newt-hype/latest/bit_flags/)
[![Build Status](https://img.shields.io/github/actions/workflow/status/sam0x17/newt-hype/ci.yaml)](https://github.com/sam0x17/newt-hype/actions/workflows/ci.yaml?query=branch%3Amain)

`newt-hype` is a small `#![no_std]` crate that makes it trivial to create newtype wrappers. It
provides two macros for defining wrappers that automatically forward many trait implementations
from the wrapped type.

## Macros
- `base_newtype!()` – declares a generic crate-level `NewType<T>` wrapper with numerous trait impls. Can
  optionally pass a name if you don't want to call it `NewType`.
- `newtype!(Alias, Inner)` or `newtype!(Alias, Base, Inner)` – creates a type alias `Alias`
  that wraps `Inner` using the chosen base newtype.

## Usage
```rust
use newt_hype::*;
// Declare the base newtype struct
base_newtype!(); // Defaults to NewType

newtype!(BetterBool, bool); // Creates a newtype struct `BetterBool` that wraps `bool` using `NewType` as the base.

let a: BetterBool = BetterBool::new(true);

assert!(*a);
assert_ne!(a, false);

newtype!(MyNewType, NewType, u32); // Creates a newtype struct `MyNewType`that wraps `u32` using `NewType` as the base
assert_eq!(MyNewType::new(42), 42);
let a = MyNewType::new(11);
let b = MyNewType::new(37);
let c = 38u32;
let d = MyNewType::new(40);
assert_eq!(a + b, MyNewType::new(48));
assert_eq!(a + c, MyNewType::new(49));
assert_eq!(d - c, MyNewType::new(2));
```

The base wrapper implements traits like `Deref`, arithmetic operators, `Clone`, `Default`, and
even iterator forwarding so your newtypes behave much like the inner types while remaining
distinct.

## Motivation
Creating a newtype is a common technique for type safety and for implementing traits on foreign
types. This crate reduces the boilerplate so you can declare a new wrapper in a single line and
opt into additional behaviour by implementing traits on the base wrapper. This abstraction is
very useful in situations where you need to newtype entire hierarchies of nested types that you
do not own.

## License
MIT OR Apache-2.0
