//! Log4j `PatternLayout` formatter engine.
//!
//! Subset of log4j conversion specifiers that maps cleanly onto the
//! `tracing` event model. The full log4j pattern grammar is large;
//! this module focuses on the most common keywords and a few
//! log4j-specific constructs that logback doesn't share:
//!
//! - `%c{1.}` / `%C{1.}` — dot-notation abbreviation (e.g.
//!   `com.example.foo.Bar` with `{1.}` → `c.e.f.Bar`).
//! - `%x` — NDC; rendered as the active span name.
//! - `%enc{sub}{html|xml|json|none}` — escape the sub-pattern
//!   before output.
//! - `%maxLen{sub}{n}` — truncate the sub-pattern to `n` chars.
//!
//! Everything else aligns with the logback engine: `%d`, `%level`,
//! `%msg`, `%ex`, `%highlight`, etc.

/// Dot-notation abbreviation for logger names (`%c{1.}`).
pub mod abbreviator;
/// Token/keyword lexer for log4j patterns.
pub mod lexer;
/// Event renderer that turns a token stream into formatted output.
pub mod renderer;

pub use lexer::{scan, Keyword, Token};
pub use renderer::Log4jFormatter;
