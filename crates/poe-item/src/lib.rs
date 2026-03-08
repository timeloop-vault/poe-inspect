//! Parse `PoE` Ctrl+Alt+C item text into structured types.
//!
//! Two-pass architecture:
//! - **Pass 1** ([`parse`]): PEST grammar + tree walker → [`RawItem`]
//! - **Pass 2** ([`resolve`]): Game data disambiguation → [`ResolvedItem`]

mod parser;
mod resolver;
pub mod types;

pub use parser::{ParseError, parse};
pub use resolver::resolve;
pub use types::{RawItem, ResolvedItem};
