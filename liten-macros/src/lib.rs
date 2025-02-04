use proc_macro::TokenStream;
use quote::ToTokens;
use syn::parenthesized;
use syn::parse::Parse;
use syn::parse_macro_input;

use syn::Block;
use syn::Ident;
use syn::ReturnType;
use syn::Token;
use syn::TypeBareFn;

#[proc_macro_attribute]
pub fn main(attrs: TokenStream, function: TokenStream) -> TokenStream {
  let func = function.clone();
  let testing = parse_macro_input!(func as MainFn);

  testing.into_token_stream().into()
}

struct MainFn {
  return_type: ReturnType,
  ident: Ident,
  block: Block,
}

impl Parse for MainFn {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    input.parse::<Token![async]>()?;
    input.parse::<Token![fn]>()?;
    let ident = input.parse::<Ident>()?;

    let content;

    parenthesized!(content in input);

    let return_type = input.parse::<ReturnType>()?;

    let block = input.parse::<Block>()?;

    Ok(MainFn { return_type, block, ident })
  }
}

impl ToTokens for MainFn {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let Self { return_type, block, ident } = self;

    let tokens_to_extend = quote::quote! {
        fn #ident() #return_type {
            liten::runtime::Runtime::new()
                .block_on(async #block)
        }
    };

    tokens.extend(tokens_to_extend);
  }
}
