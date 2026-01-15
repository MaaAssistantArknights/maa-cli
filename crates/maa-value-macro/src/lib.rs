use proc_macro::TokenStream;
use syn::parse_macro_input;

mod codegen;
mod parsing;

use parsing::{InsertMacroInput, ObjectMacroInput};

/// A convenient macro to create a MAAValue::Object
///
/// # Syntax
///
/// ```ignore
/// object!(
///     "key" => value,                              // insert(key, value)
///     "key" => value?,                             // insert(key, value.try_into()?) - propagates error
///     "key" => value??,                            // insert(key, value.try_into().unwrap()) - panics on error
///     "key" =>? value,                             // maybe_insert(key, value.map(Into::into))
///     "key" =>? value?,                            // maybe_insert(key, value.map(TryInto::try_into).transpose()?)
///     "key" =>? value??,                           // maybe_insert(key, value.map(|v| v.try_into().unwrap()))
///     "key" if "cond" == expected => value,        // conditional (Optional)
///     "key" if "c1" == e1 && "c2" == e2 => value,  // multiple conditions
/// )
/// ```
///
/// # Examples
/// ```
/// use maa_value::prelude::*;
///
/// let object = object!(
///     "bool" => true,
///     "int" => 1,
///     "float" => 1.0,
///     "string" => "string",
///     "array" => [1, 2],
///     "object" => object!(
///         "key1" => "value1",
///         "key2" => "value2",
///     ),
///     "optional" if "bool" == true => 1,
///     "optional_no_satisfied" if "bool" == false => 1,
///     "optional_no_exist" if "no_exist" == true => 1,
///     "optional_chian" if "optional" == true => 1,
/// );
/// ```
///
/// With optional values (uses `maybe_insert`, skips if `None`):
///
/// ```
/// use maa_value::prelude::*;
///
/// let optional_value: Option<i32> = Some(10);
/// let obj = object!(
///     "required" => "always",
///     "optional" =>? optional_value,  // only inserted if Some
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
///         "path" => some_path?,  // calls some_path.try_into()?, propagates error
///     ))
/// }
/// ```
///
/// With fallible conversion outside Result context (panics on error with `??`):
///
/// ```
/// use maa_value::prelude::*;
/// use std::path::Path;
///
/// let some_path = Path::new("/tmp/test");
/// let obj = object!(
///     "path" => some_path??,  // calls some_path.try_into().unwrap(), panics on error
/// );
/// ```
///
/// Combining optional and fallible conversion:
///
/// ```
/// use maa_value::{error::Result, prelude::*};
/// use std::path::PathBuf;
///
/// fn example(opt_path: Option<PathBuf>) -> Result<MAAValue> {
///     Ok(object!(
///         "path" =>? opt_path?,  // maybe_insert with try_into, propagates error
///     ))
/// }
/// ```
#[proc_macro]
pub fn object(input: TokenStream) -> TokenStream {
    parse_macro_input!(input as ObjectMacroInput)
        .generate()
        .into()
}

/// Insert entries into an existing MAAValue object
///
/// This macro allows you to insert multiple key-value pairs into an existing MAAValue object
/// using the same syntax as the `object!` macro.
///
/// # Syntax
///
/// ```ignore
/// insert!(object_expr,
///     "key" => value,                              // insert(key, value)
///     "key" => value?,                             // insert(key, value.try_into()?) - propagates error
///     "key" => value??,                            // insert(key, value.try_into().unwrap()) - panics on error
///     "key" =>? value,                             // maybe_insert(key, value.map(Into::into))
///     "key" =>? value?,                            // maybe_insert(key, value.map(TryInto::try_into).transpose()?)
///     "key" =>? value??,                           // maybe_insert(key, value.map(|v| v.try_into().unwrap()))
///     "key" if "cond" == expected => value,        // conditional (Optional)
///     "key" if "c1" == e1 && "c2" == e2 => value,  // multiple conditions
/// )
/// ```
///
/// # Examples
///
/// ```
/// use maa_value::prelude::*;
///
/// let mut obj = object!("existing" => "value");
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
/// let mut obj = object!("base" => "value");
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
