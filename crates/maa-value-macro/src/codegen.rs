//! Code generation for macro expansion

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{Expr, LitStr};

use crate::parsing::{ConversionKind, InsertKind, InsertMacroInput, ObjectEntry, ObjectMacroInput};

/// The target value type to generate code for.
pub enum TargetType {
    /// Generate a `MAAValue` (concrete, resolved values only — no Input or Optional)
    MAAValue,
    /// Generate a `MAAValueTemplate` (may contain Input and Optional variants)
    MAAValueTemplate,
}

impl TargetType {
    fn type_path(&self) -> TokenStream2 {
        match self {
            Self::MAAValue => quote! { ::maa_value::value::MAAValue },
            Self::MAAValueTemplate => quote! { ::maa_value::value::MAAValueTemplate },
        }
    }
}

impl ObjectMacroInput {
    pub fn generate(self, target: TargetType) -> TokenStream2 {
        let type_path = target.type_path();

        if self.entries.is_empty() {
            return quote! { #type_path::default() };
        }

        let object_var = Ident::new("object", Span::mixed_site());
        let object_ref_var = Ident::new("object_ref", Span::mixed_site());
        let inserts: Vec<_> = self
            .entries
            .into_iter()
            .map(|entry| entry.generate(&object_ref_var, &target))
            .collect();

        quote! {
            {
                let mut #object_var = #type_path::default();
                let #object_ref_var = &mut #object_var;
                #(#inserts)*
                #object_var
            }
        }
    }
}

impl InsertMacroInput {
    pub fn generate(self) -> TokenStream2 {
        let object_var = &self.object;

        let object_ref_var = Ident::new("object", Span::mixed_site());
        let inserts: Vec<_> = self
            .entries
            .into_iter()
            .map(|entry| entry.generate(&object_ref_var, &TargetType::MAAValueTemplate))
            .collect();

        quote! {
            {
                let #object_ref_var = &mut (#object_var);
                #(#inserts)*
            }
        }
    }
}

impl ObjectEntry {
    /// Generate the insert statement for this entry
    pub fn generate(self, object_var: &Ident, target: &TargetType) -> TokenStream2 {
        match &self.conditions {
            Some(conditions) => match target {
                TargetType::MAAValue => {
                    quote! {
                        compile_error!(
                            "conditional fields (`if` syntax) are not supported in `object!()`; \
                             use `template!()` instead"
                        );
                    }
                }
                TargetType::MAAValueTemplate => {
                    self.generate_conditional_insert(object_var, conditions)
                }
            },
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
                quote! { ::maa_value::map::MapOps::insert(#object_var, #key, #converted); }
            }
            InsertKind::Maybe => {
                let converted = self.conversion_kind.generate_option_conversion(value);
                quote! { ::maa_value::map::MapOps::maybe_insert(#object_var, #key, #converted); }
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

        match self.insert_kind {
            InsertKind::Insert => {
                let converted_value = self.conversion_kind.generate_conversion(value);
                quote! {{
                    let #value_var: ::maa_value::value::MAAValueTemplate = #converted_value;
                    ::maa_value::map::MapOps::insert(#object, #key, #optional);
                }}
            }
            InsertKind::Maybe => {
                let converted_value = self.conversion_kind.generate_conversion(&value_var);
                quote! {
                    if let ::core::option::Option::Some(#value_var) = #value {
                        let #value_var: ::maa_value::value::MAAValueTemplate = #converted_value;
                        ::maa_value::map::MapOps::insert(#object, #key, #optional);
                    }
                }
            }
        }
    }
}

/// Generate code that creates a wrapped `MAAValueTemplate::Optional` value
fn generate_optional_value(value_var: &Ident, conditions: &[(LitStr, Expr)]) -> TokenStream2 {
    let conditions_var = Ident::new("conditions", Span::mixed_site());

    let cond_inserts: Vec<_> = conditions
        .iter()
        .map(|(cond_key, expected)| {
            quote! {
                ::maa_value::map::Map::insert(
                    &mut #conditions_var,
                    ::core::convert::Into::into(#cond_key),
                    ::core::convert::Into::into(#expected)
                );
            }
        })
        .collect();

    quote! {{
        let mut #conditions_var = ::maa_value::map::Map::new();
        #(#cond_inserts)*
        ::maa_value::value::MAAValueTemplate::Optional {
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
