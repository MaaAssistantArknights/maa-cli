use proc_macro::TokenStream;
use syn::parse_macro_input;

mod codegen;
mod parsing;

use codegen::TargetType;
use parsing::{InsertMacroInput, ObjectMacroInput};

/// A convenient macro to create a [`MAAValue`](maa_value::value::MAAValue) object.
///
/// Use this macro when all values are known at compile time (no user inputs, no conditional
/// fields). For templates that may contain [`Input`](maa_value::value::MAAValueTemplate::Input)
/// or [`Optional`](maa_value::value::MAAValueTemplate::Optional) variants, use [`template!`]
/// instead.
///
/// # Syntax
///
/// ```ignore
/// object!(
///     "key" => value,           // insert(key, value)
///     "key" => value?,          // insert(key, value.try_into()?) — propagates error
///     "key" => value??,         // insert(key, value.try_into().unwrap()) — panics on error
///     "key" =>? value,          // maybe_insert(key, value.map(Into::into))
///     "key" =>? value?,         // maybe_insert with try_into, propagates error
///     "key" =>? value??,        // maybe_insert with try_into, panics on error
/// )
/// ```
///
/// Conditional fields (`if` syntax) are **not** supported — use [`template!`] for those.
///
/// # Examples
///
/// ```
/// use maa_value::prelude::*;
///
/// let value = object!(
///     "bool" => true,
///     "int" => 1,
///     "float" => 1.0,
///     "string" => "string",
///     "array" => [1, 2],
///     "nested" => object!(
///         "key1" => "value1",
///         "key2" => "value2",
///     ),
/// );
/// ```
///
/// With optional values (uses `maybe_insert`, skips if `None`):
///
/// ```
/// use maa_value::prelude::*;
///
/// let optional_value: Option<i32> = Some(10);
/// let value = object!(
///     "required" => "always",
///     "optional" =>? optional_value,
/// );
/// ```
///
/// With fallible conversion (propagates error with `?`):
///
/// ```
/// use maa_value::{error::Result, prelude::*};
///
/// fn example(some_path: &std::path::Path) -> Result<MAAValue> {
///     Ok(object!(
///         "path" => some_path?,
///     ))
/// }
/// ```
#[proc_macro]
pub fn object(input: TokenStream) -> TokenStream {
    parse_macro_input!(input as ObjectMacroInput)
        .generate(TargetType::MAAValue)
        .into()
}

/// A convenient macro to create a [`MAAValueTemplate`](maa_value::value::MAAValueTemplate) object.
///
/// Use this macro when the template may contain user inputs
/// ([`Input`](maa_value::value::MAAValueTemplate::Input),
/// [`BoolInput`](maa_value::userinput::BoolInput),
/// [`Select`](maa_value::userinput::Select)) or conditional fields (`if` syntax). Call
/// [`MAAValueTemplate::resolve()`](maa_value::value::MAAValueTemplate::resolve) to evaluate the
/// template into a concrete [`MAAValue`](maa_value::value::MAAValue).
///
/// For simple concrete objects without inputs or conditionals, use [`object!`] instead.
///
/// # Syntax
///
/// ```ignore
/// template!(
///     "key" => value,                              // insert(key, value)
///     "key" => value?,                             // insert(key, value.try_into()?) — propagates error
///     "key" => value??,                            // insert(key, value.try_into().unwrap()) — panics on error
///     "key" =>? value,                             // maybe_insert(key, value.map(Into::into))
///     "key" =>? value?,                            // maybe_insert with try_into, propagates error
///     "key" =>? value??,                           // maybe_insert with try_into, panics on error
///     "key" if "cond" == expected => value,        // conditional (Optional variant)
///     "key" if "c1" == e1 && "c2" == e2 => value, // multiple conditions
/// )
/// ```
///
/// # Examples
///
/// ```
/// use maa_value::prelude::*;
///
/// let tmpl = template!(
///     "bool" => true,
///     "optional" if "bool" == true => 1,
///     "optional_no_satisfied" if "bool" == false => 1,
///     "optional_no_exist" if "no_exist" == true => 1,
///     "optional_chain" if "optional" == true => 1,
/// );
///
/// let resolved = tmpl.resolve().unwrap();
/// assert_eq!(resolved.get("optional").unwrap().as_int(), Some(1));
/// assert!(resolved.get("optional_no_satisfied").is_none());
/// ```
///
/// With user inputs (uses default in batch mode):
///
/// ```
/// use maa_value::prelude::*;
///
/// let tmpl = template!(
///     "name" => Input::new(Some("default_name".to_string())),
///     "enabled" => BoolInput::new(Some(true)),
/// );
///
/// let resolved = tmpl.resolve().unwrap();
/// assert_eq!(resolved.get("name").unwrap().as_str(), Some("default_name"));
/// ```
///
/// With fallible conversion (propagates error with `?`):
///
/// ```
/// use maa_value::{error::Result, prelude::*};
///
/// fn example(path: &std::path::Path) -> Result<MAAValueTemplate> {
///     Ok(template!(
///         "path" => path?,
///         "debug" if "path" == "/debug" => true,
///     ))
/// }
/// ```
#[proc_macro]
pub fn template(input: TokenStream) -> TokenStream {
    parse_macro_input!(input as ObjectMacroInput)
        .generate(TargetType::MAAValueTemplate)
        .into()
}

/// Insert entries into an existing object.
///
/// This macro inserts key-value pairs into an existing
/// [`MAAValueTemplate`](maa_value::value::MAAValueTemplate) using the same syntax as
/// [`template!`]. Conditional fields (`if` syntax) are supported and produce
/// [`Optional`](maa_value::value::MAAValueTemplate::Optional) variants.
///
/// # Syntax
///
/// ```ignore
/// insert!(object_expr,
///     "key" => value,                              // insert(key, value)
///     "key" => value?,                             // insert(key, value.try_into()?) — propagates error
///     "key" => value??,                            // insert(key, value.try_into().unwrap()) — panics on error
///     "key" =>? value,                             // maybe_insert(key, value.map(Into::into))
///     "key" =>? value?,                            // maybe_insert with try_into, propagates error
///     "key" =>? value??,                           // maybe_insert with try_into, panics on error
///     "key" if "cond" == expected => value,        // conditional (Optional variant)
///     "key" if "c1" == e1 && "c2" == e2 => value, // multiple conditions
/// )
/// ```
///
/// # Examples
///
/// ```
/// use maa_value::prelude::*;
///
/// let mut obj = template!("existing" => "value");
/// insert!(obj,
///     "new_key" => "new_value",
///     "number" => 42
/// );
///
/// assert_eq!(obj.get("existing").unwrap().as_str(), Some("value"));
/// assert_eq!(obj.get("new_key").unwrap().as_str(), Some("new_value"));
/// assert_eq!(obj.get("number").unwrap().as_int(), Some(42));
/// ```
///
/// With optional values:
///
/// ```
/// use maa_value::prelude::*;
///
/// let mut obj = template!("base" => "value");
/// let optional: Option<i32> = Some(10);
/// insert!(obj,
///     "optional" =>? optional
/// );
/// assert_eq!(obj.get("optional").unwrap().as_int(), Some(10));
/// ```
#[proc_macro]
pub fn insert(input: TokenStream) -> TokenStream {
    parse_macro_input!(input as InsertMacroInput)
        .generate()
        .into()
}
