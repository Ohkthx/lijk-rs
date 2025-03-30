use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, Ident, Index, parse_macro_input};

/// Derive NetEncode, convert a struct or enum into a byte vector for network transmission.
#[proc_macro_derive(NetEncode)]
pub fn derive_net_encode(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match impl_net_encode(&ast) {
        Ok(ts) => ts,
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generates `impl NetEncoder for T` with `fn encode(&self) -> Vec<u8>`.
fn impl_net_encode(ast: &DeriveInput) -> Result<TokenStream, Error> {
    let name = &ast.ident;

    let encode_body = match &ast.data {
        // Structs: named, unnamed, and unit encoding.
        Data::Struct(data_struct) => {
            match &data_struct.fields {
                // Fields within the struct are named, like struct Foo { x: T, y: U };
                Fields::Named(fields) => {
                    let recurse = fields.named.iter().map(|f| {
                        let field_name = &f.ident;
                        quote! {
                            out.extend(self.#field_name.encode());
                        }
                    });

                    quote! { #(#recurse)* }
                }

                // Struct is tuple-like, struct Foo(T, U);
                Fields::Unnamed(fields) => {
                    let recurse = fields.unnamed.iter().enumerate().map(|(i, _)| {
                        let index = Index::from(i);
                        quote! {
                            out.extend(self.#index.encode());
                        }
                    });

                    quote! { #(#recurse)* }
                }

                // Empty struct, like struct Foo;
                Fields::Unit => {
                    quote! {}
                }
            }
        }

        // Enums: named, unnamed, and unit encoding.
        Data::Enum(data_enum) => {
            // Create a match arm for each variant.
            let arms = data_enum.variants.iter().enumerate().map(|(idx, variant)| {
                let var_ident = &variant.ident;
                let variant_idx = idx as u8; // Tag ID for the variant.

                match &variant.fields {
                    // Fields within the Enum arm are named, like enum Foo::Bar { x: T, y: U };
                    Fields::Named(fields_named) => {
                        let names: Vec<_> = fields_named
                            .named
                            .iter()
                            .map(|f| f.ident.as_ref().unwrap())
                            .collect();

                        let expansions = names.iter().map(|name| {
                            quote! {
                                out.extend(#name.encode());
                            }
                        });

                        quote! {
                            #name::#var_ident { #(#names),* } => {
                                out.push(#variant_idx);
                                #(#expansions)*
                            }
                        }
                    }

                    // Enum arm is tuple-like, enum Foo::Bar(T, U);
                    Fields::Unnamed(fields_unnamed) => {
                        let field_count = fields_unnamed.unnamed.len();
                        let vars: Vec<_> = (0..field_count)
                            .map(|i| Ident::new(&format!("f{}", i), variant.ident.span()))
                            .collect();

                        // For each field in the variant, generate code similar to `out.extend(f0.encode());`
                        let expansions = vars.iter().map(|var| {
                            quote! {
                                out.extend(#var.encode());
                            }
                        });

                        quote! {
                            #name::#var_ident(#(#vars),*) => {
                                out.push(#variant_idx);
                                #(#expansions)*
                            }
                        }
                    }

                    // Unit variant, like enum Foo { A, B, C }
                    Fields::Unit => {
                        quote! {
                            #name::#var_ident => {
                                out.push(#variant_idx);
                            }
                        }
                    }
                }
            });

            // Produce the actual match statement for the enum variants.
            quote! {
                match self {
                    #(#arms),*
                }
            }
        }

        // Unions are not supported for encoding.
        Data::Union(_) => {
            return Err(Error::new_spanned(
                &ast.ident,
                "NetEncode not implemented for unions",
            ));
        }
    };

    // Wrap that body in the final `impl NetEncoder for #name { fn encode(...) { ... } }`.
    let expanded = quote! {
        #[doc = "Automatically generated implementation of the `NetEncoder` trait."]
        #[doc = "Encodes this type into a newly allocated `Vec<u8>`."]
        #[automatically_derived]
        impl NetEncoder for #name {
            /// Encodes the value into a byte vector for network transmission.
            #[inline(always)]
            fn encode(self) -> ::std::vec::Vec<u8> {
                let mut out = ::std::vec::Vec::new();
                #encode_body
                out
            }
        }
    };

    Ok(expanded.into())
}

/// Entry point for `#[derive(NetDecode)]`.
/// Derive NetDecode, convert from Vec<u8> to a struct or enum.
#[proc_macro_derive(NetDecode)]
pub fn derive_net_decode(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match impl_net_decode(&ast) {
        Ok(ts) => ts,
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generates `impl NetDecoder for #name` that returns `(Self, usize)`.
fn impl_net_decode(ast: &DeriveInput) -> Result<TokenStream, Error> {
    let name = &ast.ident;

    let decode_body = match &ast.data {
        // Structs: named, unnamed, and unit encoding.
        Data::Struct(data_struct) => {
            match &data_struct.fields {
                // Fields within the struct are named, like struct Foo { x: T, y: U };
                Fields::Named(fields) => {
                    let names: Vec<_> = fields
                        .named
                        .iter()
                        .map(|f| f.ident.as_ref().unwrap())
                        .collect();

                    let decode_fields = names.iter().map(|fname| {
                        quote! {
                            let (temp_val, used) = NetDecoder::decode(&data[offset..])?;
                            offset += used;
                            let #fname = temp_val;
                        }
                    });

                    quote! {
                        let mut offset = 0usize;
                        #(#decode_fields)*

                        ::std::result::Result::Ok((
                            Self {
                                #(#names),*
                            },
                            offset
                        ))
                    }
                }

                // Struct is tuple-like, struct Foo(T, U);
                Fields::Unnamed(fields_unnamed) => {
                    let field_count = fields_unnamed.unnamed.len();
                    let vars: Vec<_> = (0..field_count)
                        .map(|i| quote::format_ident!("f{}", i))
                        .collect();

                    let decode_steps = vars.iter().map(|fv| {
                        quote! {
                            let (#fv, used) = NetDecoder::decode(&data[offset..])?;
                            offset += used;
                        }
                    });

                    quote! {
                        let mut offset = 0usize;
                        #(#decode_steps)*

                        ::std::result::Result::Ok((
                            Self(#(#vars),*),
                            offset
                        ))
                    }
                }

                // Empty struct, like struct Foo;
                Fields::Unit => {
                    quote! {
                        // No fields to decode, so zero bytes consumed.
                        ::std::result::Result::Ok((Self, 0))
                    }
                }
            }
        }

        // Enums: named, unnamed, and unit encoding.
        Data::Enum(data_enum) => {
            let variant_arms = data_enum.variants.iter().enumerate().map(|(idx, variant)| {
                let var_ident = &variant.ident;
                let tag_value = idx as u8;

                match &variant.fields {
                    // Fields within the Enum arm are named, like enum Foo::Bar { x: T, y: U };
                    Fields::Named(fields_named) => {
                        let idents: Vec<_> = fields_named
                            .named
                            .iter()
                            .map(|f| f.ident.as_ref().unwrap())
                            .collect();

                        let decode_fields = idents.iter().map(|ident| {
                            quote! {
                                let (temp_val, used) = NetDecoder::decode(&data[offset..])?;
                                offset += used;
                                let #ident = temp_val;
                            }
                        });

                        quote! {
                            #tag_value => {
                                #(#decode_fields)*
                                ::std::result::Result::Ok((
                                    #name::#var_ident { #(#idents),* },
                                    offset
                                ))
                            }
                        }
                    }

                    // Enum arm is tuple-like, enum Foo::Bar(T, U);
                    Fields::Unnamed(fields_unnamed) => {
                        let field_count = fields_unnamed.unnamed.len();
                        let vars: Vec<_> = (0..field_count)
                            .map(|i| quote::format_ident!("f{}", i))
                            .collect();

                        let decode_steps = vars.iter().map(|fv| {
                            quote! {
                                let (#fv, used) = NetDecoder::decode(&data[offset..])?;
                                offset += used;
                            }
                        });

                        quote! {
                            #tag_value => {
                                #(#decode_steps)*
                                ::std::result::Result::Ok((#name::#var_ident(#(#vars),*), offset))
                            }
                        }
                    }

                    // Unit variant, like enum Foo { A, B, C }
                    Fields::Unit => quote! {
                        #tag_value => {
                            // Example: MyEnum::A is unit => no extra fields, just offset.
                            ::std::result::Result::Ok((#name::#var_ident, offset))
                        }
                    },
                }
            });

            quote! {
                let mut offset = 0usize;
                if data.is_empty() {
                    return ::std::result::Result::Err(
                        crate::net::error::NetError::NetCode(
                            "No data for enum discriminant".to_string()
                        )
                    );
                }
                let tag = data[offset];
                offset += 1;

                match tag {
                    #(#variant_arms),*,
                    _ => {
                        ::std::result::Result::Err(
                            crate::net::error::NetError::NetCode(
                                format!("Unknown enum variant tag: {}", tag)
                            )
                        )
                    }
                }
            }
        }

        Data::Union(_) => {
            return Err(Error::new_spanned(
                &ast.ident,
                "NetDecode not implemented for unions",
            ));
        }
    };

    // Build final `impl NetDecoder` with `(Self, usize)`
    let expanded = quote! {
        #[doc = "Automatically generated implementation of the `NetDecoder` trait."]
        #[doc = "Decodes this type into a newly allocated `T`."]
        #[automatically_derived]
        impl NetDecoder for #name {
            #[inline(always)]
            fn decode(data: &[u8]) -> ::std::result::Result<(Self, usize), crate::net::error::NetError> {
                #decode_body
            }
        }
    };

    Ok(expanded.into())
}
