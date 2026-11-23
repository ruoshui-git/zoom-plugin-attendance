//! Pure attendance derivation engine. Inputs and outputs are plain serde values;
//! this crate deliberately has no SDK, storage, or networking dependencies.
mod engine;
mod names;
pub mod media;
mod types;

pub use engine::*;
pub use names::*;
pub use media::{media_by_participant, merge_media, MediaStats};
pub(crate) use names::{candidate_people_from_index, candidate_people_index};
pub use types::*;
