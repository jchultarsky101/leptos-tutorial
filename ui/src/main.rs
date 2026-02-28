use leptos::prelude::*;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};
use tracing_web::MakeConsoleWriter;

fn init_tracing() {
    let filter = option_env!("RUST_LOG")
        .and_then(|s| s.parse::<EnvFilter>().ok())
        .unwrap_or_else(|| EnvFilter::new("warn"));

    let fmt_layer = fmt::layer()
        .with_ansi(false) // ANSI color codes are not supported in the browser console
        .without_time() // std::time is not available in WASM
        .with_writer(MakeConsoleWriter);

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();
}

fn main() {
    init_tracing();
    leptos::mount::mount_to_body(|| view! { <p>"Hello, world!"</p> })
}
