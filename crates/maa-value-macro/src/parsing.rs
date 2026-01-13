//! Parsing logic for macro input

use proc_macro2::TokenStream as TokenStream2;
use syn::{
    Expr, LitStr, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

/// The type of value conversion
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ConversionKind {
    /// Regular conversion: `value.into()`
    Into,
    /// Try conversion: `value.try_into()?`
    TryInto,
    /// Try conversion with unwrap: `value.try_into().unwrap()`
    TryIntoUnwrap,
}

struct Convertsion {
    expr: Expr,
    kind: ConversionKind,
}

impl Parse for Convertsion {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut expr: Expr = input.parse()?;
        let mut num_try_suffix = 0u8;
        while let Expr::Try(try_expr) = expr {
            num_try_suffix += 1;
            expr = *try_expr.expr;
        }
        let kind = match num_try_suffix {
            0 => ConversionKind::Into,
            1 => ConversionKind::TryInto,
            2 => ConversionKind::TryIntoUnwrap,
            _ => return Err(syn::Error::new(input.span(), "more than two ?")),
        };
        Ok(Self { expr, kind })
    }
}

/// The type of insert operation
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InsertKind {
    /// Regular insert
    Insert,
    /// Maybe insert (optional value)
    Maybe,
}

pub struct ObjectEntry {
    pub key: LitStr,
    pub conditions: Option<Vec<(LitStr, Expr)>>,
    pub value: Expr,
    pub insert_kind: InsertKind,
    pub conversion_kind: ConversionKind,
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

        let Convertsion { expr, kind } = input.parse()?;

        Ok(ObjectEntry {
            key,
            conditions,
            value: expr,
            insert_kind,
            conversion_kind: kind,
        })
    }
}

pub struct InsertMacroInput {
    pub object: Expr,
    pub entries: Punctuated<ObjectEntry, Token![,]>,
}

impl Parse for InsertMacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let object: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let entries = Punctuated::parse_terminated(input)?;
        Ok(InsertMacroInput { object, entries })
    }
}

pub struct ObjectMacroInput {
    pub entries: Punctuated<ObjectEntry, Token![,]>,
}

impl Parse for ObjectMacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let entries = Punctuated::parse_terminated(input)?;
        Ok(ObjectMacroInput { entries })
    }
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
