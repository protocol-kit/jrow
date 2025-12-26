//! Handler procedural macro implementation
//!
//! This module contains the actual implementation of the `#[handler]` attribute macro.
//! It uses the `syn` crate to parse Rust syntax, and the `quote` crate to generate
//! new Rust code at compile time.
//!
//! # Macro Expansion Process
//!
//! The macro performs these steps:
//!
//! 1. **Parse**: Parse the input function using `syn::ItemFn`
//! 2. **Extract**: Extract function name, visibility, parameters, return type
//! 3. **Transform**: Create an inner async function with the original body
//! 4. **Wrap**: Generate a factory function that uses `from_typed_fn`
//! 5. **Quote**: Convert the transformed AST back to Rust code
//!
//! # Why This Design?
//!
//! We generate a factory function (returns `Box<dyn Handler>`) rather than
//! implementing the Handler trait directly because:
//!
//! - **Easier registration**: Can call `router.route("method", handler())`
//! - **Type inference**: Rust can infer parameter and return types automatically
//! - **Closure compatibility**: Works with the existing `from_typed_fn` infrastructure
//!
//! # Code Generation Example
//!
//! Input:
//! ```ignore
//! #[handler]
//! async fn add(params: AddParams) -> Result<i32> {
//!     Ok(params.a + params.b)
//! }
//! ```
//!
//! Generated output:
//! ```ignore
//! fn add() -> Box<dyn jrow_server::Handler> {
//!     use jrow_server::from_typed_fn;
//!
//!     async fn inner_handler(params: AddParams) -> Result<i32> {
//!         Ok(params.a + params.b)
//!     }
//!
//!     from_typed_fn(inner_handler)
//! }
//! ```

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, FnArg, ItemFn, ReturnType};

/// Implementation of the handler attribute macro
///
/// This function is called at compile time by the Rust compiler when it encounters
/// `#[handler]`. It receives the attributed function as a token stream, parses it,
/// transforms it, and returns new code to replace the original function.
///
/// # Arguments
///
/// * `input` - The token stream representing the attributed async function
///
/// # Returns
///
/// A token stream representing the generated factory function
///
/// # Implementation Notes
///
/// We preserve all function attributes (doc comments, cfg, etc.) so that the
/// generated function has the same metadata as the original. This ensures
/// documentation and conditional compilation still work correctly.
pub fn handler_impl(input: TokenStream) -> TokenStream {
    // Parse the input tokens as a function item
    // This gives us structured access to all parts of the function
    let input_fn = parse_macro_input!(input as ItemFn);

    // Extract key components we need to preserve or transform
    let fn_name = &input_fn.sig.ident;      // Function name (e.g., "add")
    let fn_vis = &input_fn.vis;              // Visibility (e.g., pub, pub(crate))
    let fn_block = &input_fn.block;          // Function body (the actual implementation)
    let fn_attrs = &input_fn.attrs;          // Attributes like #[doc], #[cfg], etc.

    // Extract the parameter type from the function signature
    // We support either one typed parameter or no parameters
    let param_type = match input_fn.sig.inputs.first() {
        Some(FnArg::Typed(pat_type)) => {
            // Found a typed parameter like `params: AddParams`
            // Extract just the type part (AddParams)
            let ty = &pat_type.ty;
            quote! { #ty }
        }
        _ => {
            // No parameters, or self parameter (which we don't support)
            // Default to unit type () which deserializes from null or omitted params
            quote! { () }
        }
    };

    // Extract the return type from the function signature
    // This determines what type the async function returns
    let return_type = match &input_fn.sig.output {
        ReturnType::Type(_, ty) => {
            // Explicit return type like `-> Result<i32>`
            quote! { #ty }
        }
        ReturnType::Default => {
            // No explicit return type, defaults to ()
            quote! { () }
        }
    };

    // Generate the replacement code
    // This is the factory function that will be called to create handlers
    let expanded = quote! {
        // Preserve all original attributes (doc comments, cfg, etc.)
        #(#fn_attrs)*
        // Keep the same visibility as the original function
        #fn_vis fn #fn_name() -> Box<dyn jrow_server::Handler> {
            // Import the typed handler factory function
            use jrow_server::from_typed_fn;

            // Create an inner async function with the original body
            // This is necessary because we need to extract the parameter type
            // separately from the handler creation logic
            async fn inner_handler(params: #param_type) -> #return_type {
                // Insert the original function body here
                // This is the user's actual implementation
                #fn_block
            }

            // Use the jrow_server helper to convert the typed async function
            // into a Box<dyn Handler> that handles JSON-RPC protocol details
            from_typed_fn(inner_handler)
        }
    };

    // Convert the generated code back to a TokenStream
    // This is what the compiler will use to replace the original function
    TokenStream::from(expanded)
}


