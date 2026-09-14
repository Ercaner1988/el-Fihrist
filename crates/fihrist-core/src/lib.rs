//! # fihrist-core
//!
//! el-Fihrist domain modelleri, hata tipleri ve temel veri hatları.
//! Mikro-derme mimarisinin temelini teşkil eder.

pub mod error;
pub mod models;

pub use error::{FihristError, Result};
pub use models::{MatchType, QueryFilter, SearchResult, Skill};
