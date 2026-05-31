//! Database layer — connection management, entities, migrations, and repositories.
//!
//! This module encapsulates all direct interaction with the SQLite database
//! (via SeaORM). Higher layers should depend on repository traits, not on raw
//! connection handles or ORM entities.

pub mod connection;
pub mod entities;
pub mod migrations;
pub mod repositories;
