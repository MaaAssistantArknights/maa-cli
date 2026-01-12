use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Expr, LitStr, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
};

/// Input for the insert! macro: object, entries...
struct InsertMacroInput {
    object: Expr,
    entries: Punctuated<ObjectEntry, Token![,]>,
}

impl Parse for InsertMacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let object: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let entries = Punctuated::parse_terminated(input)?;
        Ok(InsertMacroInput { object, entries })
    }
}

/// The type of value conversion
#[derive(Clone, Copy, PartialEq, Eq)]
enum ConversionKind {
    /// Regular conversion: `value.into()`
    Into,
    /// Try conversion: `value.try_into()?`
    TryInto,
    /// Try conversion with unwrap: `value.try_into().unwrap()`
    TryIntoUnwrap,
}

/// The type of insert operation
#[derive(Clone, Copy, PartialEq, Eq)]
enum InsertKind {
    /// Regular insert
    Insert,
    /// Maybe insert (optional value)
    Maybe,
}

/// A single entry in the object macro
struct ObjectEntry {
    key: LitStr,
    conditions: Option<Vec<(LitStr, Expr)>>,
    value: Expr,
    insert_kind: InsertKind,
    conversion_kind: ConversionKind,
}

/// Parse a condition value (expression after ==), stopping at && or =>
fn parse_condition_value(input: ParseStream) -> syn::Result<Expr> {
    // We need to parse tokens carefully to stop at && or =>
    // Use a simple approach: collect tokens until we hit && or =>
    let mut tokens = Vec::new();

    while !input.is_empty() {
        // Stop if we see && or =>
        if input.peek(Token![&&]) || input.peek(Token![=>]) {
            break;
        }

        // Parse one token tree
        tokens.push(input.parse::<proc_macro2::TokenTree>()?);
    }

    // Convert tokens back to a token stream and parse as expression
    let token_stream: TokenStream2 = tokens.into_iter().collect();
    syn::parse2(token_stream)
}

impl Parse for ObjectEntry {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: LitStr = input.parse()?;

        // Parse optional conditions: if "key1" == expr1 && "key2" == expr2
        let conditions = if input.peek(Token![if]) {
            input.parse::<Token![if]>()?;
            let mut conds = Vec::new();
            loop {
                let cond_key: LitStr = input.parse()?;
                input.parse::<Token![==]>()?;

                // Parse the expected value, but stop at && or =>
                // We need to parse carefully to not consume && as part of the expression
                let expected = parse_condition_value(input)?;
                conds.push((cond_key, expected));

                // Check if there's another condition with &&
                if input.peek(Token![&&]) {
                    input.parse::<Token![&&]>()?;
                    continue;
                }
                break;
            }
            Some(conds)
        } else {
            None
        };

        input.parse::<Token![=>]>()?;

        // Check for `=>?` (maybe_insert)
        let insert_kind = if input.peek(Token![?]) {
            input.parse::<Token![?]>()?;
            InsertKind::Maybe
        } else {
            InsertKind::Insert
        };

        let value: Expr = input.parse()?;

        // Detect conversion kind from suffix operators
        let (value, conversion_kind) = match detect_try_suffix(value) {
            (v, TrySuffix::Double) => (v, ConversionKind::TryIntoUnwrap),
            (v, TrySuffix::Single) => (v, ConversionKind::TryInto),
            (v, TrySuffix::None) => (v, ConversionKind::Into),
        };

        Ok(ObjectEntry {
            key,
            conditions,
            value,
            insert_kind,
            conversion_kind,
        })
    }
}

#[derive(PartialEq)]
enum TrySuffix {
    None,
    Single, // ?
    Double, // ??
}

/// Detect and unwrap Try suffix operators (? or ??)
fn detect_try_suffix(expr: Expr) -> (Expr, TrySuffix) {
    if let Expr::Try(outer_try) = expr {
        if let Expr::Try(inner_try) = *outer_try.expr {
            // Double try: expr??
            return (*inner_try.expr, TrySuffix::Double);
        } else {
            // Single try: expr?
            return (*outer_try.expr, TrySuffix::Single);
        }
    }
    (expr, TrySuffix::None)
}

struct ObjectMacroInput {
    entries: Punctuated<ObjectEntry, Token![,]>,
}

impl Parse for ObjectMacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let entries = Punctuated::parse_terminated(input)?;
        Ok(ObjectMacroInput { entries })
    }
}

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
/// use maa_value::{MAAValue, object};
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
/// use maa_value::{MAAValue, object};
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
/// use maa_value::{MAAValue, object, Result};
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
/// use maa_value::{MAAValue, object};
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
/// use maa_value::{MAAValue, object, Result};
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
    let input = parse_macro_input!(input as ObjectMacroInput);
    let expanded = generate_object(input);
    TokenStream::from(expanded)
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
/// use maa_value::{MAAValue, object, insert};
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
/// use maa_value::{MAAValue, object, insert};
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
    let input = parse_macro_input!(input as InsertMacroInput);
    let expanded = generate_insert_macro(input);
    TokenStream::from(expanded)
}

fn generate_object(input: ObjectMacroInput) -> TokenStream2 {
    let crate_path = quote! { ::maa_value };

    if input.entries.is_empty() {
        return quote! {
            #crate_path::MAAValue::default()
        };
    }

    let inserts: Vec<_> = input
        .entries
        .into_iter()
        .map(|entry| generate_insert(&crate_path, entry))
        .collect();

    quote! {
        {
            let mut __object = #crate_path::MAAValue::default();
            #(#inserts)*
            __object
        }
    }
}

/// Generate the insert statement for a single entry
fn generate_insert(crate_path: &TokenStream2, entry: ObjectEntry) -> TokenStream2 {
    let key = &entry.key;
    let value = &entry.value;

    match &entry.conditions {
        Some(conditions) => generate_conditional_insert(
            crate_path,
            key,
            value,
            conditions,
            entry.insert_kind,
            entry.conversion_kind,
        ),
        None => generate_simple_insert(
            crate_path,
            key,
            value,
            entry.insert_kind,
            entry.conversion_kind,
        ),
    }
}

/// Generate the conversion expression for a value
fn generate_conversion(value: &Expr, conversion_kind: ConversionKind) -> TokenStream2 {
    match conversion_kind {
        ConversionKind::Into => quote! { (#value).into() },
        ConversionKind::TryInto => quote! { (#value).try_into()? },
        ConversionKind::TryIntoUnwrap => quote! { (#value).try_into().unwrap() },
    }
}

/// Generate the conversion expression inside an Option context
fn generate_option_conversion(value: &Expr, conversion_kind: ConversionKind) -> TokenStream2 {
    match conversion_kind {
        ConversionKind::Into => quote! {
            (#value).map(::core::convert::Into::into)
        },
        ConversionKind::TryInto => quote! {
            match (#value).map(::core::convert::TryInto::try_into).transpose() {
                ::core::result::Result::Ok(v) => v,
                ::core::result::Result::Err(e) => return ::core::result::Result::Err(e.into()),
            }
        },
        ConversionKind::TryIntoUnwrap => quote! {
            (#value).map(|__v| ::core::convert::TryInto::try_into(__v).unwrap())
        },
    }
}

/// Generate a simple (non-conditional) insert statement
fn generate_simple_insert(
    _crate_path: &TokenStream2,
    key: &LitStr,
    value: &Expr,
    insert_kind: InsertKind,
    conversion_kind: ConversionKind,
) -> TokenStream2 {
    match insert_kind {
        InsertKind::Insert => {
            let converted = generate_conversion(value, conversion_kind);
            quote! {
                __object.insert(#key, #converted);
            }
        }
        InsertKind::Maybe => {
            let converted = generate_option_conversion(value, conversion_kind);
            quote! {
                __object.maybe_insert(#key, #converted);
            }
        }
    }
}

/// Generate the conversion expression for a raw identifier (used in conditional context)
fn generate_identifier_conversion(
    ident: TokenStream2,
    conversion_kind: ConversionKind,
) -> TokenStream2 {
    match conversion_kind {
        ConversionKind::Into => quote! { #ident.into() },
        ConversionKind::TryInto => quote! { ::core::convert::TryInto::try_into(#ident)? },
        ConversionKind::TryIntoUnwrap => {
            quote! { ::core::convert::TryInto::try_into(#ident).unwrap() }
        }
    }
}

/// Generate a conditional insert statement (with Optional wrapper)
fn generate_conditional_insert(
    crate_path: &TokenStream2,
    key: &LitStr,
    value: &Expr,
    conditions: &[(LitStr, Expr)],
    insert_kind: InsertKind,
    conversion_kind: ConversionKind,
) -> TokenStream2 {
    let cond_inserts: Vec<_> = conditions
        .iter()
        .map(|(cond_key, expected)| {
            quote! {
                __conditions.insert(#cond_key.into(), (#expected).into());
            }
        })
        .collect();

    // For Maybe kind, wrap everything in an if-let
    if insert_kind == InsertKind::Maybe {
        let value_conversion = generate_identifier_conversion(quote! { __val }, conversion_kind);

        return quote! {
            if let ::core::option::Option::Some(__val) = #value {
                let __val: #crate_path::MAAValue = #value_conversion;
                let mut __conditions = #crate_path::Map::new();
                #(#cond_inserts)*
                let __wrapped = #crate_path::MAAValue::Optional {
                    conditions: __conditions,
                    value: __val.into(),
                };
                __object.insert(#key, __wrapped);
            }
        };
    }

    let value_conversion = generate_conversion(value, conversion_kind);

    quote! {
        {
            let __val: #crate_path::MAAValue = #value_conversion;
            let mut __conditions = #crate_path::Map::new();
            #(#cond_inserts)*
            let __wrapped = #crate_path::MAAValue::Optional {
                conditions: __conditions,
                value: __val.into(),
            };
            __object.insert(#key, __wrapped);
        }
    }
}

fn generate_insert_macro(input: InsertMacroInput) -> TokenStream2 {
    let crate_path = quote! { ::maa_value };
    let object = &input.object;

    let inserts: Vec<_> = input
        .entries
        .into_iter()
        .map(|entry| generate_insert(&crate_path, entry))
        .collect();

    quote! {
        {
            let __object = &mut (#object);
            #(#inserts)*
        }
    }
}
