use proc_macro::TokenStream;
use quote::ToTokens;
use syn::parenthesized;
use syn::parse::Parse;
use syn::parse_macro_input;

use syn::Attribute;
use syn::Block;
use syn::FnArg;
use syn::Ident;
use syn::ReturnType;
use syn::Token;

/// Boots the runtime and runs the async code within it.
///
/// This macro transforms an async main function into one that starts the Liten runtime
/// and executes the async code within it. It should be used to annotate your main function:
///
/// # Example
/// ```ignore
/// #[liten::main]
/// async fn main() {
///     // Your async code here
/// }
/// ```
///
/// The macro expands your async main function into code that creates and starts the Liten runtime,
/// then blocks on your async main body:
///
/// ```ignore
/// fn main() {
///     liten::runtime::Runtime::single_threaded().block_on(async {
///         // Your async code here
///     })
/// }
/// ```
///
/// # Note
/// This macro is required for running async code at the top level, as Rust does not
/// natively support async main functions.
#[proc_macro_attribute]
pub fn main(_: TokenStream, function: TokenStream) -> TokenStream {
  let testing = parse_macro_input!(function as CallerFn);

  MainFn(testing).into_token_stream().into()
}

/// Boots the runtime and runs the async code within it for tests.
///
/// This macro transforms an async test function into one that starts the Liten runtime
/// and executes the async code within it. It should be used to annotate your async test functions:
///
/// # Example
/// ```ignore
/// #[liten::test]
/// async fn my_test() {
///     // Your async test code here
/// }
/// ```
///
/// The macro expands your async test function into code that creates and starts the Liten runtime,
/// then blocks on your async test body:
///
/// ```ignore
/// #[test]
/// fn my_test() {
///     liten::runtime::Runtime::default().block_on(async {
///         // Your async test code here
///     })
/// }
/// ```
///
/// # Note
/// This macro not is required for running async code in tests. The runtime can be manually started, but Rust does not
/// natively support async test functions.
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

#[proc_macro_attribute]
pub fn runtime_test(_: TokenStream, function: TokenStream) -> TokenStream {
  let testing = parse_macro_input!(function as CallerFn);

  RuntimeTestFn(testing).into_token_stream().into()
}

struct CallerFn {
  attrs: Vec<Attribute>,
  return_type: ReturnType,
  args: Vec<FnArg>,
  ident: Ident,
  block: Block,
}

struct MainFn(CallerFn);

struct TestFn(CallerFn);
struct InternalTestFn(CallerFn);
struct RuntimeTestFn(CallerFn);

impl Parse for CallerFn {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let attrs = input.call(Attribute::parse_outer)?;
    let _ = input.parse::<Token![async]>();
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

    Ok(CallerFn { attrs, return_type, args, block, ident })
  }
}

impl ToTokens for MainFn {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let CallerFn { attrs, return_type, block, ident, args } = &self.0;
    let filtered_attrs =
      attrs.iter().filter(|attr| !attr.path().is_ident("main"));
    let tokens_to_extend = quote::quote! {
        #(#filtered_attrs)*
        fn #ident(#(#args),*) #return_type {
            liten::runtime::Runtime::single_threaded()
                .block_on(async #block)
        }
    };
    tokens.extend(tokens_to_extend);
  }
}

impl ToTokens for TestFn {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let CallerFn { attrs, return_type, block, ident, args } = &self.0;
    let filtered_attrs =
      attrs.iter().filter(|attr| !attr.path().is_ident("test"));
    let tokens_to_extend = quote::quote! {
        #[test]
        #(#filtered_attrs)*
        fn #ident(#(#args),*) #return_type {
            liten::runtime::Runtime::single_threaded()
                .block_on(async #block);
        }
    };
    tokens.extend(tokens_to_extend);
  }
}

impl ToTokens for InternalTestFn {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let CallerFn { attrs, return_type, block, ident, args } = &self.0;
    let filtered_attrs =
      attrs.iter().filter(|attr| !attr.path().is_ident("internal_test"));

    let block_stmts = &block.stmts;

    let block = quote::quote! {{
      let _ = tracing::subscriber::set_global_default(tracing_subscriber::fmt().with_max_level(tracing::Level::TRACE).finish());
      #(#block_stmts)*
    }};

    let tokens_to_extend = if cfg!(loom) {
      quote::quote! {
        #[cfg(loom)]
        #[test]
        #(#filtered_attrs)*
        fn #ident(#(#args),*) #return_type {
            loom::model(|| #block)
        }
      }
    } else {
      quote::quote! {
        #[test]
        #(#filtered_attrs)*
        fn #ident(#(#args),*) #return_type #block
      }
    };
    tokens.extend(tokens_to_extend);
  }
}

impl ToTokens for RuntimeTestFn {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let CallerFn { attrs, return_type, block, ident, args } = &self.0;
    let filtered_attrs =
      attrs.iter().filter(|attr| !attr.path().is_ident("test"));

    let tokens_to_extend = if cfg!(loom) {
      quote::quote! {
        #[cfg(loom)]
        #[test]
        #(#filtered_attrs)*
        fn #ident(#(#args),*) #return_type {
          loom::model(||
            liten::runtime::Runtime::single_threaded()
              .block_on(async #block);

            liten::runtime::Runtime::multi_threaded()
              .block_on(async #block)
          )
        }
      }
    } else {
      quote::quote! {
        #[test]
        #(#filtered_attrs)*
        fn #ident(#(#args),*) #return_type {
            liten::runtime::Runtime::single_threaded()
              .block_on(async #block);

            liten::runtime::Runtime::multi_threaded()
              .block_on(async #block)
          }
      }
    };
    // let tokens_to_extend = quote::quote! {
    //     #[cfg(loom)]
    //     #[test]
    //     #(#filtered_attrs)*
    //     fn #ident(#(#args),*) #return_type {
    //         liten::runtime::Runtime::single_threaded()
    //             .block_on(async #block);
    //
    //         liten::runtime::Runtime::multi_threaded()
    //             .block_on(async #block);
    //     }
    // };
    tokens.extend(tokens_to_extend);
  }
}
