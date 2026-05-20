//! `oltur_trace` — a procedural attribute macro that logs when a function
//! starts and finishes, together with how long it ran.
//!
//! It is written with only the built-in `proc_macro` crate (no `syn`/`quote`),
//! so it adds no external dependencies.

use proc_macro::{Delimiter, Literal, TokenStream, TokenTree};

/// Code prepended to the annotated function's body. This is fixed text — the
/// only thing the macro splices in is `__oltur_trace_name`, bound just above it.
///
/// `__oltur_trace_now` formats the current time as an RFC 3339 UTC timestamp
/// (`YYYY-MM-DDThh:mm:ss.sssZ`) using only `std`. `std` exposes no calendar, so
/// the date is derived from the Unix day count with Howard Hinnant's
/// days-to-civil-date algorithm. The time is UTC because `std` offers no
/// portable way to obtain the local timezone offset.
const PRELUDE: &str = r#"
fn __oltur_trace_now() -> ::std::string::String {
    let since_epoch = ::std::time::SystemTime::now()
        .duration_since(::std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = since_epoch.as_secs() as i64;
    let millis = since_epoch.subsec_millis();
    let days = secs / 86_400;
    let tod = secs % 86_400;
    let (hour, minute, second) = (tod / 3_600, (tod % 3_600) / 60, tod % 60);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };
    ::std::format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        year, month, day, hour, minute, second, millis,
    )
}
struct __OlturTraceGuard { name: &'static str, start: ::std::time::Instant }
impl ::std::ops::Drop for __OlturTraceGuard {
    fn drop(&mut self) {
        ::std::eprintln!(
            "[{}] [oltur_trace] {} finished in {:?}",
            __oltur_trace_now(), self.name, self.start.elapsed(),
        );
    }
}
::std::eprintln!(
    "[{}] [oltur_trace] {} started",
    __oltur_trace_now(), __oltur_trace_name,
);
let __oltur_trace_guard = __OlturTraceGuard {
    name: __oltur_trace_name,
    start: ::std::time::Instant::now(),
};
"#;

/// Logs the start and finish of the annotated function.
///
/// ```ignore
/// #[oltur_trace]
/// fn main() { /* ... */ }
/// ```
///
/// Emits to stderr, each line prefixed with an RFC 3339 UTC timestamp:
///
/// ```text
/// [2026-05-20T14:23:01.044Z] [oltur_trace] main started
/// [2026-05-20T14:23:01.191Z] [oltur_trace] main finished in 2.13ms
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

    // Re-emit as: <signature> { let __oltur_trace_name = "<name>"; <PRELUDE> <body> }
    // The original body is spliced back in as a block expression, so its value
    // still becomes the function's return value.
    let mut out = String::new();
    out.push_str(&signature.to_string());
    out.push_str(" { let __oltur_trace_name: &'static str = ");
    out.push_str(&name.to_string());
    out.push(';');
    out.push_str(PRELUDE);
    out.push_str(&body.to_string());
    out.push_str(" }");

    out.parse().expect("oltur_trace generated invalid Rust")
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
