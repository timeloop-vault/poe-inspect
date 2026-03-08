//! Parse `PoE` Ctrl+Alt+C item text into structured types.

mod parser;
pub mod types;

pub use parser::{ParseError, parse};
pub use types::RawItem;
