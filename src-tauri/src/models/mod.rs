//! Domain and interchange models.
//!
//! Models are pure data structures with no business logic. They serve as the
//! shared vocabulary between the database, API, and frontend layers. This
//! module also contains format-specific models for import/export (STIX,
//! tldraw canvas, Attack Flow).

pub mod attack_flow;
pub mod canvas_format;
pub mod domain;
pub mod stix;
