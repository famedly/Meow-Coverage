//! This module groups everything needed for coverage analysis of a single run

mod helpers;
mod html;
mod lcov;
mod pull;
mod push;

pub use pull::*;
pub use push::*;
