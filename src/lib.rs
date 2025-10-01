#![warn(
    clippy::unsafe_derive_deserialize,
    clippy::cloned_instead_of_copied,
    clippy::explicit_iter_loop
)]
#![allow(clippy::too_many_arguments)]
pub mod commitment;
mod hashing;
pub mod proof;
mod schnorr;
pub mod types;
pub mod util;

pub use proof::Proof;
