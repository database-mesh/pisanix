// Copyright 2022 SphereEx Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn;

// Codec needs to be reinitialize.
fn codec_renew(ident: &Ident) -> TokenStream {
    quote!(
        Self::#ident(mut item) => {
            item.codec_mut().renew();
            item
        },
    )
}

// Convert codec, from src codec to dst codec.
fn into_codec(src_ident: &Ident, dst_ident: &Ident) -> TokenStream {
    if src_ident.to_string() == "ClientAuth" {
        quote!(
            Self::#src_ident(item) => item.map_codec(|codec| #dst_ident::with_auth_info(Some(codec))),
        )
    } else {
        quote!(
            Self::#src_ident(item) => item.map_codec(|codec| #dst_ident::with_auth_info(codec.auth_info)),
        )
    }
}

pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    // Define codec types.
    // Tuple description (Enum variant, CodecType, whether need to be reinitialize)
    let codecs: Vec<(Ident, Ident, bool)> = vec![
        ("Resultset", "ResultsetCodec", true),
        ("Stmt", "Stmt", true),
        ("Common", "CommonCodec", false),
        ("ClientAuth", "ClientAuth", false),
    ]
    .iter()
    .map(|c| (Ident::new(c.0, Span::call_site()), Ident::new(c.1, Span::call_site()), c.2))
    .collect();

    let mut token_stream = TokenStream::new();

    match input.data {
        syn::Data::Enum(data) => {
            for v in &data.variants {
                let ident_string = v.ident.to_string().to_lowercase();
                // Exclude ClientAuth.
                if ident_string == "clientauth" {
                    continue;
                }

                // Exclude current variant.
                let filter_codecs = codecs.iter().filter(|c| c.0 != v.ident).collect::<Vec<_>>();
                let curr_codec = codecs.iter().find(|c| c.0 == v.ident).unwrap();
                let return_codec_name = &curr_codec.1;

                // Convert to TokenStream.
                let mut codecs = filter_codecs
                    .iter()
                    .map(|c| into_codec(&c.0, &curr_codec.1))
                    .collect::<Vec<_>>();

                let func_name = Ident::new(&format!("into_{}", ident_string), Span::call_site());

                let curr_codec = if !curr_codec.2 {
                    into_codec(&curr_codec.0, &curr_codec.1)
                } else {
                    codec_renew(&curr_codec.0)
                };

                codecs.push(curr_codec);

                let curr_token_stream = TokenStream::from_iter(codecs.into_iter());
                let into_codec = quote! (
                    pub fn #func_name(self) -> Framed<LocalStream, #return_codec_name> {
                        match self {
                            #curr_token_stream
                        }
                    }
                );

                token_stream.extend(into_codec);
            }
        }

        _ => unreachable!(),
    };

    let ident = input.ident;
    quote!(
        impl #ident {
            #token_stream
        }
    )
    .into()
}
