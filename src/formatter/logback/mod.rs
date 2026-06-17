//! Logback pattern formatter implementation.
//!
//! Full logback conversion word support with alignment, colors, and abbreviation.

/// Logger name abbreviation algorithm (`%logger{n}`).
pub mod abbreviator;
/// Format modifier parsing and application (alignment / truncation).
pub mod align;
/// ANSI color conversion words (`%highlight`, `%clr`, `%red`, etc.).
pub mod color;
/// Java SimpleDateFormat → chrono strftime mapping.
pub mod date;
/// Token/keyword lexer for logback patterns.
pub mod lexer;
/// Event renderer that turns a token stream into formatted output.
pub mod renderer;

pub use align::FormatModifier;
pub use color::Color;
pub use lexer::{scan, Keyword, Token};
pub use renderer::{collect_error_chain_frames, EventData, LogbackFormatter};
