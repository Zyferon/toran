//! Policy subsystem: schema, loader, compiler, evaluator.
//!
//! Policies are human-authored YAML files. At load time they are
//! parsed into a [`schema::PolicyFile`], validated, and compiled into
//! a flat, indexable [`schema::CompiledPolicy`]. The evaluator is
//! pure: same input + same policy => same output, no side effects,
//! no allocation on the hot path.

pub mod compiler;
pub mod evaluator;
pub mod loader;
pub mod schema;
pub mod validator;
