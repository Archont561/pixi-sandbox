pub mod error;
pub mod hash;
pub mod inventory;
pub mod pack;
pub mod plan;
pub mod publish;
pub mod reconstruct;
pub mod verify;

#[cfg(test)]
mod reconstruct_tests;

pub fn placeholder() -> &'static str {
    "sandbox-core: library crate (KB spec, §4.1)"
}
