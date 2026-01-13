//! Code generation for macro expansion

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{Expr, LitStr};

use crate::parsing::{ConversionKind, InsertKind, InsertMacroInput, ObjectEntry, ObjectMacroInput};

impl ObjectMacroInput {
    pub fn generate(self) -> TokenStream2 {
        if self.entries.is_empty() {
            return quote! { ::maa_value::MAAValue::default() };
        }

        let object_var = Ident::new("object", Span::mixed_site());
        let inserts: Vec<_> = self
            .entries
            .into_iter()
            .map(|entry| entry.generate(&object_var))
            .collect();

        quote! {
            {
                let mut #object_var = ::maa_value::MAAValue::default();
                #(#inserts)*
                #object_var
            }
        }
    }
}

impl InsertMacroInput {
    pub fn generate(self) -> TokenStream2 {
        let object = &self.object;

        let object_var = Ident::new("object", Span::mixed_site());
        let inserts: Vec<_> = self
            .entries
            .into_iter()
            .map(|entry| entry.generate(&object_var))
            .collect();

        quote! {
            {
                let #object_var = &mut (#object);
                #(#inserts)*
            }
        }
    }
}

impl ObjectEntry {
    /// Generate the insert statement for this entry
    pub fn generate(self, object_var: &Ident) -> TokenStream2 {
        match &self.conditions {
            Some(conditions) => self.generate_conditional_insert(object_var, conditions),
            None => self.generate_simple_insert(object_var),
        }
    }

    /// Generate a simple (non-conditional) insert statement
    fn generate_simple_insert(&self, object_var: &Ident) -> TokenStream2 {
        let key = &self.key;
        let value = &self.value;

        match self.insert_kind {
            InsertKind::Insert => {
                let converted = self.conversion_kind.generate_conversion(value);
                quote! {
                    #object_var.insert(#key, #converted);
                }
            }
            InsertKind::Maybe => {
                let converted = self.conversion_kind.generate_option_conversion(value);
                quote! {
                    #object_var.maybe_insert(#key, #converted);
                }
            }
        }
    }

    /// Generate a conditional insert statement (with Optional wrapper)
    fn generate_conditional_insert(
        &self,
        object: &Ident,
        conditions: &[(LitStr, Expr)],
    ) -> TokenStream2 {
        let key = &self.key;
        let value = &self.value;

        let value_var = Ident::new("val", Span::mixed_site());

        let optional = generate_optional_value(&value_var, conditions);

        // Generate the value with Optional wrapper
        match self.insert_kind {
            InsertKind::Insert => {
                // Regular insert with conditions: directly convert and wrap
                let converted_value = self.conversion_kind.generate_conversion(value);
                quote! {
                    {
                        let #value_var: ::maa_value::MAAValue = #converted_value;
                        #object.insert(#key, #optional);
                    }
                }
            }
            InsertKind::Maybe => {
                // Maybe insert with conditions: extract Option first, then convert and wrap
                let converted_value = self.conversion_kind.generate_conversion(&value_var);
                quote! {
                    if let ::core::option::Option::Some(#value_var) = #value {
                        let #value_var: ::maa_value::MAAValue = #converted_value;
                        #object.insert(#key, #optional);
                    }
                }
            }
        }
    }
}

/// Generate code that create wrapped optional value and insert
fn generate_optional_value(value_var: &Ident, conditions: &[(LitStr, Expr)]) -> TokenStream2 {
    let conditions_var = Ident::new("conditions", Span::mixed_site());

    let cond_inserts: Vec<_> = conditions
        .iter()
        .map(|(cond_key, expected)| {
            quote! {
                #conditions_var.insert(
                    ::core::convert::Into::into(#cond_key),
                    ::core::convert::Into::into(#expected)
                );
            }
        })
        .collect();

    quote! {{
        let mut #conditions_var = ::maa_value::Map::new();
        #(#cond_inserts)*
        ::maa_value::MAAValue::Optional {
            conditions: #conditions_var,
            value: ::core::convert::Into::into(#value_var),
        }
    }}
}

impl ConversionKind {
    /// Generate conversion code for any token stream (expression or identifier)
    fn generate_conversion(self, tokens: impl quote::ToTokens) -> TokenStream2 {
        match self {
            ConversionKind::Into => quote! { ::core::convert::Into::into(#tokens) },
            ConversionKind::TryInto => quote! { ::core::convert::TryInto::try_into(#tokens)? },
            ConversionKind::TryIntoUnwrap => {
                quote! { ::core::result::Result::unwrap(::core::convert::TryInto::try_into(#tokens)) }
            }
        }
    }

    /// Generate conversion code inside an Option context
    fn generate_option_conversion(self, value: &Expr) -> TokenStream2 {
        let some_var = Ident::new("some_var", Span::mixed_site());
        let conversion = self.generate_conversion(&some_var);
        quote! {
            if let ::core::option::Option::Some(#some_var) = #value {
                ::core::option::Option::Some(#conversion)
            } else {
                ::core::option::Option::None
            }
        }
    }
}
