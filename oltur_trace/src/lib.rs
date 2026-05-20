//! `oltur_trace` — a procedural attribute macro that logs when a function
//! starts and finishes, together with how long it ran.
//!
//! It is written with only the built-in `proc_macro` crate (no `syn`/`quote`),
//! so it adds no external dependencies.

use proc_macro::{Delimiter, Literal, TokenStream, TokenTree};

/// Logs the start and finish of the annotated function.
///
/// ```ignore
/// #[oltur_trace]
/// fn main() { /* ... */ }
/// ```
///
/// Emits to stderr:
///
/// ```text
/// [oltur_trace] main started
/// [oltur_trace] main finished in 2.13ms
/// ```
///
/// The "finished" line is printed from a `Drop` guard, so it fires on every
/// exit path — an early `return`, a `?`, or a panic unwinding through the
/// function.
#[proc_macro_attribute]
pub fn oltur_trace(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let tokens: Vec<TokenTree> = item.into_iter().collect();

    // The function name is the identifier right after the `fn` keyword.
    let fn_name = tokens
        .iter()
        .position(|t| matches!(t, TokenTree::Ident(i) if i.to_string() == "fn"))
        .and_then(|i| tokens.get(i + 1))
        .map(ToString::to_string);

    // A function item ends with its body: a brace-delimited group.
    let has_body = matches!(
        tokens.last(),
        Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace
    );

    let Some(fn_name) = fn_name else {
        return reject(tokens, "#[oltur_trace] can only be applied to a function");
    };
    if !has_body {
        return reject(tokens, "#[oltur_trace] expects a function with a body");
    }

    // Split the item into its signature and its trailing body group.
    let (signature, body) = tokens.split_at(tokens.len() - 1);
    let signature: TokenStream = signature.iter().cloned().collect();
    let body = &body[0];
    let name = Literal::string(&fn_name);

    // Re-emit the function with a tracing guard prepended to its body. The
    // original body is spliced back in as a block expression, so its value
    // still becomes the function's return value.
    format!(
        "{signature} {{ \
            struct __OlturTraceGuard {{ name: &'static str, start: ::std::time::Instant }} \
            impl ::std::ops::Drop for __OlturTraceGuard {{ \
                fn drop(&mut self) {{ \
                    ::std::eprintln!(\
                        \"[oltur_trace] {{}} finished in {{:?}}\", \
                        self.name, self.start.elapsed()) \
                }} \
            }} \
            ::std::eprintln!(\"[oltur_trace] {{}} started\", {name}); \
            let __oltur_trace_guard = __OlturTraceGuard {{ \
                name: {name}, start: ::std::time::Instant::now() \
            }}; \
            {body} \
        }}"
    )
    .parse()
    .expect("oltur_trace generated invalid Rust")
}

/// Emits a `compile_error!` while keeping the original item, so the only error
/// the user sees is the helpful one.
fn reject(item: Vec<TokenTree>, message: &str) -> TokenStream {
    let mut out: TokenStream = format!("compile_error!({message:?});")
        .parse()
        .expect("valid compile_error invocation");
    out.extend(item);
    out
}
