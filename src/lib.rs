#![warn(
    clippy::pedantic,
    clippy::cargo,
    rustdoc::all,
    missing_docs,
    future_incompatible,
    rust_2018_idioms
)]

mod allocation;
mod allocator;
mod buffer;
mod slab;
mod slabring;
pub use self::{
    allocation::Allocation,
    allocator::{Allocator, Config},
    buffer::Buffer,
};
