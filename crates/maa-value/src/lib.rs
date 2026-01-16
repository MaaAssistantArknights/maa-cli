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
//! 1. **[`value::MAAValue`]**: Represents configuration *templates* that may contain:
//!    - User input fields (`Input`, `BoolInput`, `Select`)
//!    - Conditional fields (`Optional`) that depend on other fields
//!    - Regular values (primitives, arrays, objects)
//!
//! 2. **[`value::ResolvedMAAValue`]**: Represents *resolved* configuration containing only concrete
//!    data after all user inputs have been collected and conditions evaluated.
//!
//! The transformation from template to resolved value happens via the
//! [`value::MAAValue::resolve()`] method, which:
//! - Queries the user for any required inputs
//! - Evaluates conditional dependencies
//! - Produces a final concrete configuration
//!
//! ## Quick Start
//!
//! ```
//! use maa_value::prelude::*;
//!
//! // Create a configuration template
//! let config = object!(
//!     "name" => "my-app",
//!     "debug" => BoolInput::new(Some(false)),  // User input with default
//!     "log_level" if "debug" == true => "verbose"  // Conditional field
//! );
//!
//! // Resolve it (in this case, uses defaults without prompting)
//! let resolved = config.resolve().unwrap();
//!
//! // Access resolved values
//! assert_eq!(resolved.get("name").unwrap().as_str(), Some("my-app"));
//! assert_eq!(resolved.get("debug").unwrap().as_bool(), Some(false));
//! assert!(resolved.get("log_level").is_none());  // Not included (debug is false)
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
//!
//! let config = object!(
//!     "username" => Input::new(Some("admin".to_string())),
//!     "auto_update" => BoolInput::new(Some(true)),
//!     "theme" => SelectD::from_iter(["light", "dark"], 1.try_into().ok()).unwrap()
//! );
//! ```
//!
//! When [`value::MAAValue::resolve()`] is called, these inputs use their default
//! values in batch mode or prompt the user in interactive mode.
//!
//! ### Conditional Fields
//!
//! Fields can be conditionally included based on the values of other fields:
//!
//! ```
//! use maa_value::prelude::*;
//!
//! let config = object!(
//!     "mode" => "production",
//!     "debug_port" if "mode" == "development" => 9229  // Only included if mode is development
//! );
//!
//! let resolved = config.resolve().unwrap();
//! assert!(resolved.get("debug_port").is_none());  // Not included
//! ```
//!
//! ### Type Safety
//!
//! The type system ensures you can't accidentally use an unresolved template where a
//! resolved value is expected:
//!
//! - **[`value::MAAValue`]**: Implements `Deserialize` (load from config files)
//! - **[`value::ResolvedMAAValue`]**: Implements `Serialize` (save resolved configs)
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
//! ┌─────────────┐
//! │   MAAValue  │  ◄── Contains Input, Optional variants
//! │  (Template) │
//! └────────┬────┘
//!          │ resolve()
//!          │ • Query user inputs
//!          │ • Evaluate conditionals
//!          │ • Topological sort dependencies
//!          ▼
//! ┌─────────────────┐
//! │ ResolvedMAAValue│  ◄── Contains only concrete data
//! │   (Concrete)    │
//! └────────┬────────┘
//!          │ Serialize / Be-Deserialize
//!          ▼
//! ┌──────────────┐
//! │  JSON String │
//! │   or Struct  │
//! └──────────────┘
//! ```
//!
//! ## Module Organization
//!
//! - [`value`]: Core [`value::MAAValue`] and [`value::ResolvedMAAValue`] types
//! - [`de`]: Deserializer implementation for direct struct conversion
//! - [`map`]: Map operations trait ([`MapOps`](map::MapOps))
//! - [`array`]: Array operations trait ([`ArrayOps`](array::ArrayOps))
//! - [`convert`]: Type conversion traits ([`AsPrimitive`](convert::AsPrimitive),
//!   [`TryAs`](convert::TryAs))
//! - [`input`]: Input value definitions
//! - [`userinput`]: User input types and traits
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
pub mod userinput;
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
/// let resolved = config.resolve().unwrap();
/// assert_eq!(resolved.get("name").unwrap().as_str(), Some("example"));
/// ```
///
/// # Exports
///
/// ## Macros
///
/// - [`insert!`](maa_value_macro::insert): Macro for inserting values into objects with optional
///   value support
/// - [`object!`](maa_value_macro::object): Macro for creating [`MAAValue`](crate::value::MAAValue)
///   objects with a clean syntax
///
/// ## Core Types
///
/// - [`MAAValue`](crate::value::MAAValue): Unresolved values that may contain user inputs and
///   conditional fields
/// - [`ResolvedMAAValue`](crate::value::ResolvedMAAValue): Fully resolved values containing only
///   concrete data
/// - [`MAAPrimitive`](crate::primitive::MAAPrimitive): Primitive value types (bool, int, float,
///   string)
/// - [`MAAInput`](crate::input::MAAInput): Input value definitions
///
/// ## User Input Types
///
/// - [`Input`](crate::userinput::Input): Generic user input with default value
/// - [`BoolInput`](crate::userinput::BoolInput): Boolean user input
/// - [`Select`](crate::userinput::Select): Selection from alternatives
/// - [`Selectable`](crate::userinput::Selectable): Trait for selectable types
/// - [`UserInput`](crate::userinput::UserInput): Trait for user input types
/// - [`ValueWithDesc`](crate::userinput::ValueWithDesc): Value with description
/// - [`SelectD`](crate::userinput::SelectD): type alias for `Select<ValueWithDesc<_>>`
///
/// ## Traits
///
/// - [`ArrayOps`](crate::array::ArrayOps): Operations on array-like values
/// - [`MapOps`](crate::map::MapOps): Operations on map-like values (get, insert, merge, etc.)
/// - [`AsPrimitive`](crate::convert::AsPrimitive): Convert to primitive types
/// - [`TryAs`](crate::convert::TryAs): Try to convert to a specific type
pub mod prelude {
    pub use maa_value_macro::{insert, object};

    pub use crate::{
        array::ArrayOps,
        convert::{AsPrimitive, TryAs},
        input::MAAInput,
        map::MapOps,
        primitive::MAAPrimitive,
        userinput::{BoolInput, Input, Select, SelectD, Selectable, UserInput, ValueWithDesc},
        value::{MAAValue, ResolvedMAAValue},
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
