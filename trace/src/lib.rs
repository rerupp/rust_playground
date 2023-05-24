//! Function tracing macro.
//! 
//! This library provides a procedural attribute macro that can be used to log function entry points.
//! The concept is to allow a program execution to be traced. This is handy when you have an issue
//! with performace and you are trying to identify bottlenecks.
//!   
extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, Stmt};

/// The function attribute for tracing code execution.
///
/// The attribute can be added to any function although there is an issue with standalone `struct` functions
/// where the entire pathname is not available. 
#[proc_macro_attribute]
pub fn trace(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    let mut item_fn = parse_macro_input!(input as ItemFn);
    let ident = item_fn.sig.ident.to_string();
    // check to see if the function is from a struct instance
    let trace_ts: TokenStream = match item_fn.sig.inputs.first() {
        Some(fn_arg) => match fn_arg {
            syn::FnArg::Receiver(_) => struct_trace(&ident),
            syn::FnArg::Typed(_) => standalone_trace(&ident),
        },
        None => standalone_trace(&ident),
    };
    // eprintln!("log fn {}", fn_tokens.to_string());
    let stmt: Stmt = parse_macro_input!(trace_ts as Stmt);
    item_fn.block.stmts.insert(0, stmt);
    // eprintln!("Resulting ItemFn {}", quote!(#item_fn).to_string());
    TokenStream::from(quote!(#item_fn))
}

/// Adds logging to a standalone function.
/// 
/// The following statement is returned as a token stream.
/// 
/// `log::trace!("{}::{} Enter", module_path!(), <function name>);`
/// 
/// where `<function name>` is the functions name.
/// 
fn standalone_trace(ident: &str) -> TokenStream {
    let log_enter: TokenStream = quote!(
        log::trace!("{}::{} Enter", module_path!(), #ident);
    )
    .into();
    log_enter
}

/// Add logging to a `struct` instance function.
/// 
/// The following code block is returned as a token stream.
/// 
/// ```text
/// {
///    fn type_name<T: ?Sized + ::std::any::Any>(_: &T) -> &'static str {
///        std::any::type_name::<T>()
///    }
///    log::trace!("{}.{} - Enter", type_name(self), <function name>);
///}
/// ```
/// 
/// where `<function name>` is the functions name.
/// 
fn struct_trace(ident: &str) -> TokenStream {
    let log_enter = quote!({
        fn type_name<T: ?Sized + ::std::any::Any>(_: &T) -> &'static str {
            std::any::type_name::<T>()
        }
        log::trace!("{}.{} - Enter", type_name(self), #ident);
    })
    .into();
    log_enter
}

#[cfg(test)]
mod tests {
    use super::quote;
    use proc_macro2::TokenStream;
    use std::str::FromStr;
    use syn::parse2;

    #[test]
    fn trace_example() {
        let f = r#"
pub fn test_me(s: &str) -> String {
    let string = String::from(s);
    string
}"#;
        let ts = TokenStream::from_str(f).unwrap();
        eprintln!("{}", ts.to_string());
        if let syn::Item::Fn(mut item_fn) = syn::parse2(ts).unwrap() {
            let log_trace = quote!(log_trace!("enter"););
            // eprintln!("{}", log_trace.to_string());
            item_fn.block.stmts.insert(0, syn::parse2(log_trace).unwrap());
            let _ts: TokenStream = quote!(#item_fn).into();
            // eprintln!("{}", ts.to_string());
        } else {
            panic!("Did not get an ItemFn!!!");
        }
    }
    #[test]
    fn struct_fn() {
        let test_case = r#"
        // pub struct TestCase;
        impl TestCase {
            pub fn member_fn(&self, s: &str) -> String {
                String::from(s)
            }
            pub fn struct_fn(s: &str) -> String {
                String::from(s)
            }
        }
        "#;
        let ts = TokenStream::from_str(test_case).unwrap();
        eprintln!("{ts}");
        match parse2(ts) as syn::Result<syn::Item> {
            // Ok(item) => eprintln!("{:#?}", item),
            Ok(_item) => (),
            Err(error) => eprintln!("{:?}", error),
        }
    }
    // eprintln!("{}", output.to_string());
    // eprintln!("module path: {}", module_path!());
    // match syn::parse2(output) as syn::Result<syn::Item> {
    //     Ok(item) => {
    //         match item {
    //             syn::Item::Const(_) => eprintln!("Item::Const"),
    //             syn::Item::Enum(_) => eprintln!("Item::Enum"),
    //             syn::Item::ExternCrate(_) => eprintln!("Item::ExternCrate"),
    //             syn::Item::Fn(item_fn) => {
    //                 eprintln!("{:#?}", item_fn);
    //             }
    //             syn::Item::ForeignMod(_) => eprintln!("Item::ForeignMod"),
    //             syn::Item::Impl(_) => eprintln!("Item::Impl"),
    //             syn::Item::Macro(_) => eprintln!("Item::Macro"),
    //             syn::Item::Macro2(_) => eprintln!("Item::Macro2"),
    //             syn::Item::Mod(_) => eprintln!("Item::Mod"),
    //             syn::Item::Static(_) => eprintln!("Item::Static"),
    //             syn::Item::Struct(_) => eprintln!("Item::Struct"),
    //             syn::Item::Trait(_) => eprintln!("Item::Trait"),
    //             syn::Item::TraitAlias(_) => eprintln!("Item::TraitAlias"),
    //             syn::Item::Type(_) => eprintln!("Item::Type"),
    //             syn::Item::Union(_) => eprintln!("Item::Union"),
    //             syn::Item::Use(_) => eprintln!("Item::Use"),
    //             syn::Item::Verbatim(_) => eprintln!("Item::Verbatim"),
    //             _ => eprintln!("Item variant unknown!!!"),
    //         }
    //     },
    //     Err(error) => eprintln!("{error}"),
    // };
    // eprintln!("{:?}", item_fn);
}
