//! Pure attendance derivation engine. Inputs and outputs are plain serde values;
//! this crate deliberately has no SDK, storage, or networking dependencies.
mod engine;
mod names;
mod types;

pub use engine::*;
pub use names::*;
pub use types::*;
