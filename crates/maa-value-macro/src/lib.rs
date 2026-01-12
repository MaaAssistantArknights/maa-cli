use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Expr, LitStr, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
};

/// The type of insert operation
#[derive(Clone, Copy, PartialEq, Eq)]
enum InsertKind {
    /// Regular insert: `"key" => value`
    Insert,
    /// Try insert (fallible conversion): `"key" => value?`
    Try,
    /// Maybe insert (optional value): `"key" =>? value`
    Maybe,
}

/// A single entry in the object macro
struct ObjectEntry {
    key: LitStr,
    conditions: Option<Vec<(LitStr, Expr)>>,
    value: Expr,
    insert_kind: InsertKind,
}

/// Unwrap a Try expression (expr?) and return the inner expression
/// Returns (inner_expr, true) if it was a Try expression, (original_expr, false) otherwise
fn unwrap_try_expr(expr: Expr) -> (Expr, bool) {
    if let Expr::Try(try_expr) = expr {
        (*try_expr.expr, true)
    } else {
        (expr, false)
    }
}

impl Parse for ObjectEntry {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: LitStr = input.parse()?;

        // Parse optional conditions: if "key1" == expr1, "key2" == expr2
        let conditions = if input.peek(Token![if]) {
            input.parse::<Token![if]>()?;
            let mut conds = Vec::new();
            loop {
                let cond_key: LitStr = input.parse()?;
                input.parse::<Token![==]>()?;
                let expected: Expr = input.parse()?;
                conds.push((cond_key, expected));
                if input.peek(Token![,]) && !input.peek2(Token![=>]) {
                    // This comma is between conditions, not before =>
                    // We need to check if the next token after comma is a string literal
                    // (condition key) or if it's followed by =>
                    let lookahead = input.fork();
                    lookahead.parse::<Token![,]>().ok();
                    if lookahead.peek(LitStr) && !lookahead.peek2(Token![=>]) {
                        input.parse::<Token![,]>()?;
                        continue;
                    }
                }
                break;
            }
            Some(conds)
        } else {
            None
        };

        input.parse::<Token![=>]>()?;

        // Check for `=>?` (maybe_insert)
        let is_maybe = input.peek(Token![?]);
        if is_maybe {
            input.parse::<Token![?]>()?;
        }

        let value: Expr = input.parse()?;

        // Check if the expression ends with `?` (Try expression)
        // The parser will have consumed `value?` as Expr::Try, so we need to unwrap it
        let (value, is_try) = if is_maybe {
            (value, false)
        } else {
            unwrap_try_expr(value)
        };

        let insert_kind = if is_maybe {
            InsertKind::Maybe
        } else if is_try {
            InsertKind::Try
        } else {
            InsertKind::Insert
        };

        Ok(ObjectEntry {
            key,
            conditions,
            value,
            insert_kind,
        })
    }
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
///     "key" => value,                              // insert()
///     "key" => value?,                             // try_insert() - propagates Result error
///     "key" =>? value,                             // maybe_insert() - skips if None
///     "key" if "cond" == expected => value,        // conditional (Optional)
///     "key" if "c1" == e1, "c2" == e2 => value,    // multiple conditions
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
/// With fallible conversion (uses `try_insert`, propagates error with `?`):
///
/// ```
/// use maa_value::{MAAValue, object, Result};
///
/// fn example(some_path: &std::path::Path) -> Result<MAAValue> {
///     Ok(object!(
///         "path" => some_path?,  // calls try_insert, propagates error
///     ))
/// }
/// ```
#[proc_macro]
pub fn object(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ObjectMacroInput);
    let expanded = generate_object(input);
    TokenStream::from(expanded)
}

fn generate_object(input: ObjectMacroInput) -> TokenStream2 {
    // Use absolute path with `::` prefix to avoid shadowing by local variables
    let crate_path = quote! { ::maa_value };

    if input.entries.is_empty() {
        return quote! {
            #crate_path::MAAValue::default()
        };
    }

    let mut inserts = Vec::new();

    for entry in input.entries {
        let key = &entry.key;
        let value = &entry.value;

        let insert_stmt = if let Some(conditions) = &entry.conditions {
            // Conditional insert with Optional wrapper
            let cond_inserts: Vec<_> = conditions
                .iter()
                .map(|(cond_key, expected)| {
                    quote! {
                        __conditions.insert(#cond_key.into(), (#expected).into());
                    }
                })
                .collect();

            match entry.insert_kind {
                InsertKind::Maybe => {
                    // Conditional with maybe (skip if None)
                    quote! {
                        if let ::core::option::Option::Some(__val) = #value {
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
                InsertKind::Try => {
                    // Conditional with try (fallible conversion)
                    quote! {
                        {
                            let __val: #crate_path::MAAValue = (#value).try_into()?;
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
                InsertKind::Insert => {
                    // Conditional without modifiers (regular insert)
                    quote! {
                        {
                            let __val = #value;
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
            }
        } else {
            match entry.insert_kind {
                InsertKind::Maybe => {
                    // Non-conditional maybe_insert
                    quote! {
                        __object.maybe_insert(#key, #value);
                    }
                }
                InsertKind::Try => {
                    // Non-conditional try_insert
                    quote! {
                        __object.try_insert(#key, #value)?;
                    }
                }
                InsertKind::Insert => {
                    // Non-conditional regular insert
                    quote! {
                        __object.insert(#key, #value);
                    }
                }
            }
        };

        inserts.push(insert_stmt);
    }

    quote! {
        {
            let mut __object = #crate_path::MAAValue::default();
            #(#inserts)*
            __object
        }
    }
}
