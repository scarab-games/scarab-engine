#![warn(missing_docs)]
#![feature(drain_filter)]

//! The Scarab Engine Macros Library
///
/// more documentation coming soon
use std::env;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};
use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, Path, PathSegment, Result};

fn in_self() -> bool {
    env::var("CARGO_PKG_NAME").unwrap() == "scarab_engine"
}

fn scarab_root() -> Path {
    if in_self() {
        Path {
            leading_colon: None,
            segments: ["crate"]
                .iter()
                .map(|&s| Ident::new(s, Span::call_site()))
                .map(PathSegment::from)
                .collect(),
        }
    } else {
        Path {
            leading_colon: Some(Default::default()),
            segments: ["scarab_engine"]
                .iter()
                .map(|&s| Ident::new(s, Span::call_site()))
                .map(PathSegment::from)
                .collect(),
        }
    }
}

#[proc_macro_derive(HasBox, attributes(has_box))]
/// Derives the HasBox trait for types
pub fn derive_has_box(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let implementation;
    match impl_call_fn_on_marked_field(&input, Ident::new("get_box", Span::call_site()), "has_box")
    {
        Ok(i) => implementation = i,
        Err(e) => return e.into_compile_error().into(),
    };

    let name = input.ident;
    let root = scarab_root();

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote_spanned! { Span::call_site() =>
        impl #impl_generics #root ::types::physbox::HasBox for #name #ty_generics #where_clause {
            fn get_box(&self) -> &#root ::types::physbox::PhysBox {
                #implementation
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(HasBoxMut, attributes(has_box))]
/// Derives the HasBoxMut trait for types
pub fn derive_has_box_mut(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let implementation;

    match impl_call_fn_on_marked_field(
        &input,
        Ident::new("get_box_mut", Span::call_site()),
        "has_box",
    ) {
        Ok(i) => implementation = i,
        Err(e) => return e.into_compile_error().into(),
    };

    let name = input.ident;
    let root = scarab_root();

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote_spanned! { Span::call_site() =>
        impl #impl_generics #root ::types::physbox::HasBoxMut for #name #ty_generics #where_clause {
            fn get_box_mut(&mut self) -> &mut #root ::types::physbox::PhysBox {
                #implementation
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(HasSolidity, attributes(has_solidity))]
/// Derives the HasSolidity trait for types
pub fn derive_has_solidity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let implementation;

    match impl_call_fn_on_marked_field(
        &input,
        Ident::new("get_solidity", Span::call_site()),
        "has_solidity",
    ) {
        Ok(i) => implementation = i,
        Err(e) => return e.into_compile_error().into(),
    };

    let name = input.ident;
    let root = scarab_root();

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote_spanned! { Span::call_site() =>
        impl #impl_generics #root ::types::HasSolidity for #name #ty_generics #where_clause {
            fn get_solidity(&self) -> & #root ::types::Solidity {
                #implementation
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(HasHealth, attributes(has_health))]
/// Derives the HasSolidity trait for types
pub fn derive_has_health(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let health_impl;
    let health_impl_mut;

    match impl_call_fn_on_marked_field(
        &input,
        Ident::new("get_health", Span::call_site()),
        "has_health",
    ) {
        Ok(i) => health_impl = i,
        Err(e) => return e.into_compile_error().into(),
    };

    match impl_call_fn_on_marked_field(
        &input,
        Ident::new("get_health_mut", Span::call_site()),
        "has_health",
    ) {
        Ok(i) => health_impl_mut = i,
        Err(e) => return e.into_compile_error().into(),
    };

    let name = input.ident;
    let root = scarab_root();

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote_spanned! { Span::call_site() =>
        impl #impl_generics #root ::types::HasHealth for #name #ty_generics #where_clause {
            fn get_health(&self) -> & #root ::types::Health {
                #health_impl
            }

            fn get_health_mut(&mut self) -> &mut #root ::types::Health {
                #health_impl_mut
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(HasUuid, attributes(has_uuid))]
/// Derives the HasSolidity trait for types
pub fn derive_has_uuid(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let implementation;

    match impl_call_fn_on_marked_field(&input, Ident::new("uuid", Span::call_site()), "has_uuid") {
        Ok(i) => implementation = i,
        Err(e) => return e.into_compile_error().into(),
    };

    let name = input.ident;
    let root = scarab_root();

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote_spanned! { Span::call_site() =>
        impl #impl_generics #root ::types::HasUuid for #name #ty_generics #where_clause {
            fn uuid(&self) -> #root ::types::Uuid {
                #implementation
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(HasEntity, attributes(has_entity))]
/// Derives the HasEntity trait for types
pub fn derive_has_entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let entity_impl;
    let entity_impl_mut;

    match impl_call_fn_on_marked_field(
        &input,
        Ident::new("get_entity", Span::call_site()),
        "has_entity",
    ) {
        Ok(i) => entity_impl = i,
        Err(e) => return e.into_compile_error().into(),
    };

    match impl_call_fn_on_marked_field(
        &input,
        Ident::new("get_entity_mut", Span::call_site()),
        "has_entity",
    ) {
        Ok(i) => entity_impl_mut = i,
        Err(e) => return e.into_compile_error().into(),
    };

    let name = input.ident;
    let root = scarab_root();

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote_spanned! { Span::call_site() =>
        impl #impl_generics #root ::gameobject::entity::HasEntity for #name #ty_generics #where_clause {
            fn get_entity(&self) -> & #root ::gameobject::entity::Entity {
                #entity_impl
            }

            fn get_entity_mut(&mut self) -> &mut #root ::gameobject::entity::Entity {
                #entity_impl_mut
            }
        }
    };

    TokenStream::from(expanded)
}

fn impl_call_fn_on_marked_field(
    input: &DeriveInput,
    fn_name: Ident,
    attr_name: &str,
) -> Result<TokenStream2> {
    match input.data {
        Data::Struct(ref data) => {
            let mut parts_with_box = Vec::new();
            match data.fields {
                Fields::Named(ref fields) => {
                    for field in &fields.named {
                        parts_with_box.append(
                            &mut field
                                .attrs
                                .iter()
                                .filter(|a| a.path().is_ident(attr_name))
                                .map(|_a| {
                                    // For now simply push the field's name
                                    // later it would be useful to be able to configure additional accessors
                                    field.ident.as_ref().unwrap().to_owned()
                                })
                                .collect(),
                        )
                    }
                }
                Fields::Unnamed(ref _fields) => {
                    return Err(syn::Error::new(
                        Span::call_site(),
                        format!("Can't derive '{attr_name}' on a tuple struct"),
                    ));
                }
                Fields::Unit => {
                    return Err(syn::Error::new(
                        Span::call_site(),
                        format!("Can't derive '{attr_name}' on a unit struct"),
                    ));
                }
            }
            if parts_with_box.len() != 1 {
                return Err(syn::Error::new(
                    Span::call_site(),
                    format!("Can't derive on a struct with more than one field marked {attr_name}"),
                ));
            }

            let accessor = parts_with_box.pop().unwrap();
            // TODO! this is terrible for hygeine. Need to specify <X as Trait> for the function call
            Ok(quote! { self.#accessor.#fn_name () })
        }
        Data::Enum(ref data) => {
            let v = data.variants.iter().map(|v| &v.ident);

            Ok(quote! {
                match self {
                    #( Self::#v x=> x.#fn_name () ),*
                }
            })
        }
        Data::Union(ref _data) => {
            return Err(syn::Error::new(
                Span::call_site(),
                format!("Can't derive '{attr_name}' on a union"),
            ));
        }
    }
}
