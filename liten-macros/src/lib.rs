use proc_macro::TokenStream;
use quote::ToTokens;
use syn::parenthesized;
use syn::parse::Parse;
use syn::parse_macro_input;

use syn::Block;
use syn::FnArg;
use syn::Ident;
use syn::ReturnType;
use syn::Token;

#[proc_macro_attribute]
pub fn main(_: TokenStream, function: TokenStream) -> TokenStream {
  let testing = parse_macro_input!(function as CallerFn);

  MainFn(testing).into_token_stream().into()
}

#[proc_macro_attribute]
pub fn test(_: TokenStream, function: TokenStream) -> TokenStream {
  let func = function.clone();
  let testing = parse_macro_input!(func as CallerFn);

  TestFn(testing).into_token_stream().into()
}

#[proc_macro_attribute]
pub fn internal_test(_: TokenStream, function: TokenStream) -> TokenStream {
  let testing = parse_macro_input!(function as CallerFn);

  InternalTestFn(testing).into_token_stream().into()
}

struct CallerFn {
  return_type: ReturnType,
  args: Vec<FnArg>,
  ident: Ident,
  block: Block,
}

struct MainFn(CallerFn);

struct TestFn(CallerFn);
struct InternalTestFn(CallerFn);

impl Parse for CallerFn {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    input.parse::<Token![async]>()?;
    input.parse::<Token![fn]>()?;
    let ident = input.parse::<Ident>()?;

    let _content;
    parenthesized!(_content in input);

    let mut args = Vec::new();
    loop {
      if _content.is_empty() {
        break;
      }

      args.push(_content.parse::<FnArg>()?);
    }

    let return_type = input.parse::<ReturnType>()?;

    let block = input.parse::<Block>()?;

    Ok(CallerFn { return_type, args, block, ident })
  }
}

impl ToTokens for MainFn {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let CallerFn { return_type, block, ident, args } = &self.0;

    let tokens_to_extend = quote::quote! {
        fn #ident(#(#args),*) #return_type {
            liten::runtime::Runtime::new()
                .block_on(async #block)
        }
    };

    tokens.extend(tokens_to_extend);
  }
}

impl ToTokens for TestFn {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let CallerFn { return_type, block, ident, args } = &self.0;

    let tokens_to_extend = quote::quote! {
        #[test]
        fn #ident(#(#args),*) #return_type {
            liten::runtime::Runtime::new()
                .block_on(async #block)
        }
    };

    tokens.extend(tokens_to_extend);
  }
}

impl ToTokens for InternalTestFn {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let CallerFn { return_type, block, ident, args } = &self.0;

    let tokens_to_extend = quote::quote! {
        #[test]
        fn #ident(#(#args),*) #return_type {
            crate::runtime::Runtime::new()
                .block_on(async #block)
        }
    };

    tokens.extend(tokens_to_extend);
  }
}
