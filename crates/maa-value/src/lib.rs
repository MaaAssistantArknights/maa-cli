#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
//! # maa-value
//!
//! A type-safe value system for configuration templates with user input support and
//! conditional fields.
//!
//! ## Overview
//!
//! This crate provides a two-stage value system designed for interactive configuration:
//!
//! 1. **[`value::MAAValueTemplate`]**: Represents configuration *templates* that may contain:
//!    - User input fields (`Inquiry`, `Confirm`, `Select`)
//!    - Conditional fields (`Optional`) that depend on other fields
//!    - Regular values (primitives, arrays, objects)
//!
//! 2. **[`value::MAAValue`]**: Represents *resolved* configuration containing only concrete data
//!    after all user inputs have been collected and conditions evaluated.
//!
//! The transformation from template to resolved value happens via
//! [`value::MAAValueTemplate::resolved_by()`] or
//! [`value::MAAValueTemplate::resolved_by()`], which:
//! - Queries the user for any required inputs
//! - Evaluates conditional dependencies
//! - Produces a final concrete configuration
//!
//! ## Quick Start
//!
//! ```
//! use maa_value::prelude::*;
//! use maa_question::prelude::{BatchResolver, Confirm};
//!
//! // Create a configuration template with user input and a conditional field
//! let config = template!(
//!     "name" => "my-app",
//!     "debug" => Confirm::new(false),  // User input with default
//!     "log_level" if "debug" == true => "verbose"  // Conditional field
//! );
//!
//! // Resolve it without prompting
//! let mut resolver = BatchResolver::default();
//! let resolved = config.resolved_by(&mut resolver).unwrap();
//!
//! // Access resolved values
//! assert_eq!(resolved.get("name").unwrap().as_str(), Some("my-app"));
//! assert_eq!(resolved.get("debug").unwrap().as_bool(), Some(false));
//! assert!(resolved.get("log_level").is_none());  // Not included (debug is false)
//! ```
//!
//! For simple concrete objects without user inputs or conditionals, use
//! [`object!`](`maa_value_macro::object`) directly:
//!
//! ```
//! use maa_value::prelude::*;
//!
//! let value = object!("name" => "my-app", "count" => 42);
//! assert_eq!(value.get("name").unwrap().as_str(), Some("my-app"));
//! ```
//!
//! ## Key Concepts
//!
//! ### User Inputs
//!
//! Templates can include fields that should be filled by user input:
//!
//! ```
//! use maa_value::prelude::*;
//! use std::num::NonZero;
//! use maa_question::prelude::{Confirm, Inquiry, SelectD};
//!
//! let config = template!(
//!     "username" => Inquiry::new("admin".to_string()),
//!     "auto_update" => Confirm::new(true),
//!     "theme" => SelectD::<String>::from_iter(["light", "dark"], NonZero::new(1).unwrap()).unwrap()
//! );
//! ```
//!
//! When [`value::MAAValueTemplate::resolved_by()`] is called, these inputs are answered by the
//! provided resolver.
//!
//! ### Conditional Fields
//!
//! Fields can be conditionally included based on the values of other fields:
//!
//! ```
//! use maa_value::prelude::*;
//!
//! let config = template!(
//!     "mode" => "production",
//!     "debug_port" if "mode" == "development" => 9229  // Only included if mode is development
//! );
//!
//! use maa_question::prelude::BatchResolver;
//!
//! let mut resolver = BatchResolver::default();
//! let resolved = config.resolved_by(&mut resolver).unwrap();
//! assert!(resolved.get("debug_port").is_none());  // Not included
//! ```
//!
//! ### Type Safety
//!
//! The type system ensures you can't accidentally use an unresolved template where a
//! resolved value is expected:
//!
//! - **[`value::MAAValueTemplate`]**: Implements `Deserialize` (load from config files),
//! - **[`value::MAAValue`]**: Implements `Serialize` and `Deserialize` (load and write a generic
//!   value).
//!
//! This prevents bugs where templates might be used directly without resolution.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────┐
//! │   Config File    │
//! │ (JSON/YAML/TOML) │
//! └────────┬─────────┘
//!          │ Deserialize
//!          ▼
//! ┌──────────────────┐
//! │ MAAValueTemplate │  ◄── Contains Input, Optional variants
//! └────────┬─────────┘
//!          │ resolved_by(...)
//!          │ • Query user inputs
//!          │ • Evaluate conditionals
//!          │ • Topological sort dependencies
//!          ▼
//! ┌──────────────┐
//! │   MAAValue   │  ◄── Contains only concrete data
//! └──────┬───────┘
//!        │ Serialize / De-Deserialize
//!        ▼
//! ┌──────────────┐
//! │  JSON String │
//! │   or Struct  │
//! └──────────────┘
//! ```
//!
//! ## Module Organization
//!
//! - [`value`]: Core [`value::MAAValueTemplate`] and [`value::MAAValue`] types
//! - [`de`]: Deserializer implementation for direct struct conversion
//! - [`map`]: Map operations trait ([`MapOps`](map::MapOps))
//! - [`mod@array`]: Array operations trait ([`ArrayOps`](array::ArrayOps))
//! - [`convert`]: Type conversion traits ([`AsPrimitive`](convert::AsPrimitive),
//!   [`TryAs`](convert::TryAs))
//! - [`input`]: Question value definitions
//! - [`maa_question`]: Question/form types and traits (external crate)
//! - [`primitive`]: Primitive value types
//! - [`error`]: Error types
//! - [`prelude`]: Common imports for convenience
//!
//! ## Feature Flags
//!
//! - `schema`: Enable JSON schema generation support via `schemars`

// Allow the proc-macro to reference types via `maa_value::` path even inside this crate
extern crate self as maa_value;

pub mod array;
pub mod convert;
pub mod de;
pub mod error;
pub mod input;
pub mod map;
pub mod primitive;
pub mod value;

/// Convenience re-exports for common types and traits.
///
/// This module provides a single import point for the most commonly used items from the
/// `maa-value` crate. Import this module to quickly get started without having to enumerate
/// individual imports.
///
/// # Usage
///
/// ```
/// use maa_value::prelude::*;
///
/// // Now you have access to all the common types and traits
/// let config = object!(
///     "name" => "example",
///     "count" => 42,
///     "enabled" => true
/// );
///
/// assert_eq!(config.get("name").unwrap().as_str(), Some("example"));
/// ```
///
/// # Exports
///
/// ## Macros
///
/// - [`object!`](maa_value_macro::object): Create a concrete [`MAAValue`] object (no user inputs or
///   conditional fields)
/// - [`template!`](maa_value_macro::template): Create a [`MAAValueTemplate`] that may contain user
///   inputs and conditional fields
/// - [`insert!`](maa_value_macro::insert): Insert entries into an existing object
///
/// ## Core Types
///
/// - [`MAAValueTemplate`]: Configuration templates that may contain user inputs and conditional
///   fields
/// - [`MAAValue`]: Fully resolved values containing only concrete data
/// - [`MAAPrimitive`](crate::primitive::MAAPrimitive): Primitive value types (bool, int, float,
///   string)
/// - [`MAAInput`](crate::input::MAAInput): Question value definitions
///
/// ## Traits
///
/// - [`ArrayOps`](crate::array::ArrayOps): Operations on array-like values
/// - [`MapOps`](crate::map::MapOps): Operations on map-like values (get, insert, merge, etc.)
/// - [`AsPrimitive`](crate::convert::AsPrimitive): Convert to primitive types
/// - [`TryAs`](crate::convert::TryAs): Try to convert to a specific type
///
/// Related question types such as [`Inquiry`](maa_question::Inquiry),
/// [`Confirm`](maa_question::Confirm), and [`SelectD`](maa_question::SelectD)
/// are provided by the `maa-question` crate.
///
/// [`MAAValue`]: crate::value::MAAValue
/// [`MAAValueTemplate`]: crate::value::MAAValueTemplate
pub mod prelude {
    pub use maa_value_macro::{insert, object, template};

    pub use crate::{
        array::ArrayOps,
        convert::{AsPrimitive, TryAs},
        input::MAAInput,
        map::MapOps,
        primitive::MAAPrimitive,
        value::{MAAValue, MAAValueTemplate},
    };
}

/// Represents the result of attempting to convert or extract a value.
///
/// This enum is similar to [`Result`] but used when the operation isn't truly an error case—
/// instead, it represents whether a conversion succeeded or whether the original value should
/// be preserved. This is commonly used in methods that attempt to extract inner values from
/// wrapper types.
///
/// # Variants
///
/// - [`Value(V)`](Outcome::Value): The operation succeeded and produced a value of type `V`.
/// - [`Original(O)`](Outcome::Original): The operation couldn't proceed (e.g., wrong variant), so
///   the original value of type `O` is returned unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Outcome<V, O> {
    /// The operation succeeded, producing a value of type `V`.
    Value(V),
    /// The operation couldn't proceed; the original value of type `O` is returned.
    Original(O),
}
