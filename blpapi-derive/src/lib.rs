extern crate proc_macro;

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{parse_macro_input, parse_quote, Data, DeriveInput, Fields, GenericParam, Generics};

#[proc_macro_derive(RefData)]
pub fn derive_ref_data(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: DeriveInput = parse_macro_input!(input as DeriveInput);

    let name: syn::Ident = input.ident;

    let generics: Generics = add_trait_bounds(input.generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let fields: TokenStream = fields(&input.data);
    let on_field: TokenStream = on_field(&input.data);

    let expanded: TokenStream = quote! {
        impl #impl_generics blpapi::ref_data::RefData for #name #ty_generics #where_clause {
            #fields
            #on_field
        }
    };

    proc_macro::TokenStream::from(expanded)
}

fn add_trait_bounds(mut generics: Generics) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param
                .bounds
                .push(parse_quote!(blpapi::ref_data::RefData));
        }
    }

    generics
}

fn on_field(data: &Data) -> TokenStream {
    match data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let recurse = fields.named.iter().map(|f| {
                    let name: &Option<syn::Ident> = &f.ident;
                    let field: String = f.ident.as_ref().unwrap().to_string().to_uppercase();

                    quote_spanned! {f.span()=>
                        #field => if let Some(v) = element.get_at(0) {
                            self.#name = v;
                        },
                    }
                });
                quote! {
                    fn on_field(&mut self, field: &str, element: &blpapi::element::Element) {
                        match field {
                            #(#recurse)*
                            _ => { dbg!("Unrecognized field '{}'...", field); }
                        }
                    }
                }
            }
            _ => unimplemented!(),
        },
        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    }
}

fn fields(data: &Data) -> TokenStream {
    match data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let recurse = fields.named.iter().map(|f: &syn::Field| {
                    let field = f.ident.as_ref().unwrap().to_string().to_uppercase();
                    quote_spanned! {f.span()=> #field }
                });

                quote! {
                    const FIELDS: &'static [&'static str] = &[#(#recurse),*];
                }
            }

            _ => unimplemented!(),
        },

        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    }
}
