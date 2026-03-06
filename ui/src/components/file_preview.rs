use leptos::prelude::*;
use pulldown_cmark::{Event, Options, Parser, html as cm_html};

use crate::api;
use crate::app::{ModalState, PreviewTarget};
use crate::error::UiError;

// ── Preview kind classification ───────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum PreviewKind {
    Image,
    Markdown,
    Text,
    Csv,
    Stl,
    Unsupported,
}

fn classify(content_type: &str, filename: &str) -> PreviewKind {
    let lower_name = filename.to_ascii_lowercase();

    if content_type.starts_with("image/") {
        return PreviewKind::Image;
    }
    // STL 3D model — checked before text because ASCII STL files may be
    // served as text/plain, and the extension is the most reliable signal.
    if content_type == "model/stl"
        || content_type == "application/sla"
        || content_type == "application/vnd.ms-pki.stl"
        || lower_name.ends_with(".stl")
    {
        return PreviewKind::Stl;
    }
    // CSV — checked before generic text/ so `.csv` files aren't shown as raw text.
    if content_type == "text/csv" || lower_name.ends_with(".csv") {
        return PreviewKind::Csv;
    }
    if content_type == "text/markdown"
        || content_type == "text/x-markdown"
        || lower_name.ends_with(".md")
        || lower_name.ends_with(".markdown")
    {
        return PreviewKind::Markdown;
    }
    if content_type.starts_with("text/")
        || content_type.contains("json")
        || content_type.contains("xml")
        || content_type.contains("javascript")
        || content_type.contains("typescript")
        || content_type.contains("yaml")
        || content_type.contains("toml")
    {
        return PreviewKind::Text;
    }
    PreviewKind::Unsupported
}

// ── Markdown rendering ────────────────────────────────────────────────────────

/// Convert Markdown to sanitized HTML.
///
/// Defence in depth:
/// 1. Raw HTML events are stripped at the parser level.
/// 2. The rendered HTML is passed through `ammonia` before touching the DOM.
fn render_markdown(input: &str) -> String {
    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES;

    // Strip raw HTML blocks and inline HTML from the event stream.
    let parser = Parser::new_ext(input, opts)
        .filter(|e| !matches!(e, Event::Html(_) | Event::InlineHtml(_)));

    let mut html_buf = String::new();
    cm_html::push_html(&mut html_buf, parser);

    // Second pass: sanitize with ammonia to remove any remaining dangerous tags
    // or attributes that slipped through (e.g., event handlers).
    ammonia::clean(&html_buf)
}

// ── STL Three.js interop ──────────────────────────────────────────────────────

/// Call `window.__stlPreview.init(container, stlUrl)` via JS interop.
/// Sets `stl_active` to `true` on success; logs a warning on failure.
fn init_stl_scene(container: &web_sys::HtmlDivElement, stl_url: &str, stl_active: RwSignal<bool>) {
    let Some(win) = web_sys::window() else {
        return;
    };
    let Ok(preview_obj) = js_sys::Reflect::get(&win, &"__stlPreview".into()) else {
        return;
    };
    if preview_obj.is_undefined() {
        return;
    }
    let Ok(init_val) = js_sys::Reflect::get(&preview_obj, &"init".into()) else {
        return;
    };
    if !init_val.is_function() {
        return;
    }
    let init_fn = js_sys::Function::from(init_val);

    let container_val = wasm_bindgen::JsValue::from(container.clone());
    let url_val = wasm_bindgen::JsValue::from_str(stl_url);
    let Ok(promise_val) = init_fn.call2(&preview_obj, &container_val, &url_val) else {
        return;
    };

    let future = wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(promise_val));
    wasm_bindgen_futures::spawn_local(async move {
        match future.await {
            Ok(_) => stl_active.set(true),
            Err(e) => {
                tracing::warn!(
                    "STL scene init failed: {:?}",
                    e.as_string().unwrap_or_default()
                );
            }
        }
    });
}

/// Call `window.__stlPreview.dispose()` via JS interop. Safe to call any time.
fn dispose_stl_scene() {
    call_stl_method("dispose");
}

/// Call a zero-argument method on `window.__stlPreview` by name.
fn call_stl_method(method: &str) {
    let Some(win) = web_sys::window() else {
        return;
    };
    let Ok(preview_obj) = js_sys::Reflect::get(&win, &"__stlPreview".into()) else {
        return;
    };
    if preview_obj.is_undefined() {
        return;
    }
    let Ok(fn_val) = js_sys::Reflect::get(&preview_obj, &method.into()) else {
        return;
    };
    if fn_val.is_function() {
        let func = js_sys::Function::from(fn_val);
        let _ = func.call0(&preview_obj);
    }
}

// ── CSV parsing & helpers ─────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct CsvData {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortDir {
    Asc,
    Desc,
}

fn parse_csv(input: &str) -> Result<CsvData, String> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(input.as_bytes());

    let headers: Vec<String> = rdr
        .headers()
        .map_err(|e| format!("failed to read CSV headers: {e}"))?
        .iter()
        .map(String::from)
        .collect();

    let col_count = headers.len();
    let mut rows = Vec::new();
    for result in rdr.records() {
        let record = result.map_err(|e| format!("CSV row parse error: {e}"))?;
        let mut row: Vec<String> = record.iter().map(String::from).collect();
        // Pad short rows to match header count.
        row.resize(col_count, String::new());
        rows.push(row);
    }

    Ok(CsvData { headers, rows })
}

// ── API base (kept in sync with api.rs) ───────────────────────────────────────

const API_BASE: &str = "http://localhost:3000";

// ── Component ─────────────────────────────────────────────────────────────────

#[component]
pub fn FilePreview() -> impl IntoView {
    let preview_file =
        use_context::<RwSignal<Option<PreviewTarget>>>().expect("preview_file context missing");
    let modal = use_context::<RwSignal<Option<ModalState>>>().expect("modal context missing");
    let _ = modal; // available for future preview actions

    // Fetch text content reactively whenever preview_file changes.
    // Returns None for image/stl/unsupported (no fetch needed) or while target is None.
    let text_resource: LocalResource<Option<Result<String, UiError>>> =
        LocalResource::new(move || {
            let target = preview_file.get();
            async move {
                let t = target?;
                let kind = classify(&t.content_type, t.path.name());
                match kind {
                    PreviewKind::Text | PreviewKind::Markdown | PreviewKind::Csv => {
                        Some(api::fetch_file_content(&t.path).await)
                    }
                    PreviewKind::Image | PreviewKind::Stl | PreviewKind::Unsupported => None,
                }
            }
        });

    // NodeRef for the markdown container — innerHTML is set by the Effect below.
    let md_ref: NodeRef<leptos::html::Div> = NodeRef::new();

    // Whenever rendered Markdown HTML changes, inject it into the DOM.
    // We use a NodeRef + Effect instead of `prop:innerHTML` so Leptos does not
    // escape the HTML string.
    Effect::new(move |_| {
        let target = preview_file.get();
        let html = match (target, text_resource.map(|r| r.clone())) {
            (Some(t), Some(Some(Ok(ref text))))
                if classify(&t.content_type, t.path.name()) == PreviewKind::Markdown =>
            {
                render_markdown(text)
            }
            _ => String::new(),
        };
        if let Some(el) = md_ref.get() {
            el.set_inner_html(&html);
        }
    });

    // ── STL state ─────────────────────────────────────────────────────────────
    let stl_container_ref: NodeRef<leptos::html::Div> = NodeRef::new();
    let stl_active: RwSignal<bool> = RwSignal::new(false);

    // Unified STL lifecycle: dispose synchronously on every preview_file
    // change, then — if the new target is STL — defer init to the next
    // microtask so Leptos can reconcile the DOM first.  This avoids both
    // the on_load-doesn't-re-fire problem and the Effect-races-on_load bug.
    Effect::new(move |_| {
        let target = preview_file.get();

        // Always dispose + reset (both are idempotent no-ops when inactive).
        dispose_stl_scene();
        stl_active.set(false);

        if let Some(t) = &target
            && classify(&t.content_type, t.path.name()) == PreviewKind::Stl
        {
            let stl_url = format!(
                "{API_BASE}/files/{}",
                t.path.as_str().trim_start_matches('/')
            );
            // Yield one microtask so Leptos flushes the DOM.  By the time
            // this runs, stl_container_ref points at the mounted element.
            wasm_bindgen_futures::spawn_local(async move {
                let _ = wasm_bindgen_futures::JsFuture::from(js_sys::Promise::resolve(
                    &wasm_bindgen::JsValue::UNDEFINED,
                ))
                .await;
                if let Some(el) = stl_container_ref.get() {
                    let div: &web_sys::HtmlDivElement = &el;
                    init_stl_scene(div, &stl_url, stl_active);
                }
            });
        }
    });

    view! {
        // ── Panel header ──────────────────────────────────────────────────────
        <div class="flex-shrink-0 flex items-center justify-between \
                     px-3 py-2 border-b border-gray-100 min-h-[40px]">
            <span class="text-xs font-medium text-gray-500 truncate">
                {move || {
                    preview_file
                        .get()
                        .map(|t| t.path.name().to_owned())
                        .unwrap_or_default()
                }}
            </span>
            <button
                class="ml-2 flex-shrink-0 text-gray-400 hover:text-gray-700 \
                       focus:outline-none transition-colors"
                on:click=move |_| preview_file.set(None)
                title="Close preview"
            >
                <span class="material-symbols-outlined" style="font-size:18px;">"close"</span>
            </button>
        </div>

        // ── Panel body ────────────────────────────────────────────────────────
        {move || {
            let Some(target) = preview_file.get() else {
                return view! { <div></div> }.into_any();
            };
            let kind = classify(&target.content_type, target.path.name());

            match kind {
                // ── Image ─────────────────────────────────────────────────────
                PreviewKind::Image => {
                    let src = format!(
                        "{API_BASE}/files/{}",
                        target.path.as_str().trim_start_matches('/')
                    );
                    view! {
                        <div class="flex-1 overflow-auto flex items-center \
                                    justify-center p-4 bg-gray-50">
                            <img
                                src=src
                                alt=target.path.name().to_owned()
                                class="max-w-full max-h-full object-contain \
                                       rounded shadow-sm"
                            />
                        </div>
                    }
                    .into_any()
                }

                // ── STL 3D model ──────────────────────────────────────────────
                PreviewKind::Stl => {
                    let is_loading = move || !stl_active.get();
                    view! {
                        <div class="flex-1 flex flex-col overflow-hidden bg-gray-50 dark:bg-gray-800">
                            // Canvas area (grows to fill).
                            <div class="flex-1 relative overflow-hidden min-h-0">
                                // Loading overlay — shown until JS scene reports ready.
                                <Show when=is_loading>
                                    <div class="absolute inset-0 flex items-center justify-center \
                                                bg-gray-50/80 dark:bg-gray-800/80 z-10">
                                        <div class="flex flex-col items-center gap-2">
                                            <span class="material-symbols-outlined text-gray-300 \
                                                         animate-spin" style="font-size:32px;">
                                                "progress_activity"
                                            </span>
                                            <span class="text-xs text-gray-400">
                                                "Loading 3D model\u{2026}"
                                            </span>
                                        </div>
                                    </div>
                                </Show>
                                // Three.js appends its <canvas> inside this div.
                                <div
                                    node_ref=stl_container_ref
                                    class="w-full h-full"
                                />
                            </div>
                            // Controls toolbar — shown once loaded.
                            <Show when=move || stl_active.get()>
                                <div class="flex-shrink-0 border-t border-gray-200 dark:border-gray-700 \
                                            bg-white dark:bg-gray-800 px-2 py-1.5 flex items-center \
                                            justify-center gap-1">
                                    <button
                                        class="p-1 rounded hover:bg-gray-100 text-gray-500 \
                                               hover:text-gray-700 transition-colors"
                                        title="Zoom in"
                                        on:click=move |_| call_stl_method("zoomIn")
                                    >
                                        <span class="material-symbols-outlined" style="font-size:18px;">
                                            "zoom_in"
                                        </span>
                                    </button>
                                    <button
                                        class="p-1 rounded hover:bg-gray-100 text-gray-500 \
                                               hover:text-gray-700 transition-colors"
                                        title="Zoom out"
                                        on:click=move |_| call_stl_method("zoomOut")
                                    >
                                        <span class="material-symbols-outlined" style="font-size:18px;">
                                            "zoom_out"
                                        </span>
                                    </button>
                                    <div class="w-px h-4 bg-gray-200 mx-1" />
                                    <button
                                        class="p-1 rounded hover:bg-gray-100 text-gray-500 \
                                               hover:text-gray-700 transition-colors"
                                        title="Auto-rotate"
                                        on:click=move |_| call_stl_method("toggleAutoRotate")
                                    >
                                        <span class="material-symbols-outlined" style="font-size:18px;">
                                            "3d_rotation"
                                        </span>
                                    </button>
                                    <div class="w-px h-4 bg-gray-200 mx-1" />
                                    <button
                                        class="p-1 rounded hover:bg-gray-100 text-gray-500 \
                                               hover:text-gray-700 transition-colors"
                                        title="Reset view"
                                        on:click=move |_| call_stl_method("resetView")
                                    >
                                        <span class="material-symbols-outlined" style="font-size:18px;">
                                            "restart_alt"
                                        </span>
                                    </button>
                                </div>
                            </Show>
                        </div>
                    }
                    .into_any()
                }

                // ── CSV table ─────────────────────────────────────────────────
                PreviewKind::Csv => {
                    match text_resource.map(|r| r.clone()) {
                        None | Some(None) => view! {
                            <div class="flex-1 flex items-center justify-center">
                                <span class="material-symbols-outlined \
                                             text-gray-300 animate-spin"
                                    style="font-size:32px;">
                                    "progress_activity"
                                </span>
                            </div>
                        }
                        .into_any(),

                        Some(Some(Err(UiError::FileTooLarge(_)))) => view! {
                            <div class="flex-1 flex flex-col items-center \
                                        justify-center p-6 text-center text-gray-400">
                                <span class="material-symbols-outlined"
                                    style="font-size:40px; display:block; margin-bottom:8px;">
                                    "data_usage"
                                </span>
                                <p class="text-sm">"File exceeds the 1 MiB preview limit."</p>
                            </div>
                        }
                        .into_any(),

                        Some(Some(Err(ref e))) => {
                            let msg = e.to_string();
                            view! {
                                <div class="flex-1 flex items-center justify-center \
                                            p-4 text-center text-red-500 text-sm">
                                    {msg}
                                </div>
                            }
                            .into_any()
                        }

                        Some(Some(Ok(ref text))) => {
                            match parse_csv(text) {
                                Err(msg) => view! {
                                    <div class="flex-1 flex items-center justify-center \
                                                p-4 text-center text-red-500 text-sm">
                                        {msg}
                                    </div>
                                }
                                .into_any(),

                                Ok(csv_data) => {
                                    let col_count = csv_data.headers.len();
                                    let row_count = csv_data.rows.len();

                                    // ── Sort state ───────────────────────────────
                                    let sort_col: RwSignal<Option<usize>> = RwSignal::new(None);
                                    let sort_dir: RwSignal<SortDir> = RwSignal::new(SortDir::Asc);

                                    // ── Column widths (pixels) ───────────────────
                                    let default_w = 150.0_f64;
                                    let col_widths: RwSignal<Vec<f64>> =
                                        RwSignal::new(vec![default_w; col_count]);

                                    // ── Column resize state ──────────────────────
                                    let resize_col: RwSignal<Option<usize>> = RwSignal::new(None);
                                    let resize_x0: RwSignal<f64> = RwSignal::new(0.0);
                                    let resize_w0: RwSignal<f64> = RwSignal::new(0.0);

                                    let headers = csv_data.headers.clone();
                                    let rows_data = csv_data.rows.clone();

                                    view! {
                                        <div class="flex-1 flex flex-col overflow-hidden">
                                            // Drag overlay during column resize.
                                            <Show when=move || resize_col.get().is_some()>
                                                <div
                                                    class="fixed inset-0 z-50 cursor-col-resize"
                                                    on:mousemove=move |e: web_sys::MouseEvent| {
                                                        if let Some(ci) = resize_col.get_untracked() {
                                                            let dx = e.client_x() as f64 - resize_x0.get_untracked();
                                                            let new_w = (resize_w0.get_untracked() + dx).max(50.0);
                                                            col_widths.update(|ws| {
                                                                if let Some(w) = ws.get_mut(ci) {
                                                                    *w = new_w;
                                                                }
                                                            });
                                                        }
                                                    }
                                                    on:mouseup=move |_| {
                                                        resize_col.set(None);
                                                    }
                                                />
                                            </Show>

                                            // Scrollable table wrapper.
                                            <div class="flex-1 overflow-auto min-h-0">
                                                <table class="csv-table">
                                                    <colgroup>
                                                        {(0..col_count)
                                                            .map(|i| {
                                                                view! {
                                                                    <col style=move || {
                                                                        let w = col_widths.get().get(i).copied().unwrap_or(default_w);
                                                                        format!("width:{w}px;min-width:{w}px;")
                                                                    } />
                                                                }
                                                            })
                                                            .collect::<Vec<_>>()}
                                                    </colgroup>
                                                    <thead>
                                                        <tr>
                                                            {headers
                                                                .iter()
                                                                .enumerate()
                                                                .map(|(i, h)| {
                                                                    let header = h.clone();
                                                                    view! {
                                                                        <th>
                                                                            <div
                                                                                class="csv-th-inner"
                                                                                on:click=move |_| {
                                                                                    let cur = sort_col.get_untracked();
                                                                                    if cur == Some(i) {
                                                                                        match sort_dir.get_untracked() {
                                                                                            SortDir::Asc => sort_dir.set(SortDir::Desc),
                                                                                            SortDir::Desc => {
                                                                                                sort_col.set(None);
                                                                                                sort_dir.set(SortDir::Asc);
                                                                                            }
                                                                                        }
                                                                                    } else {
                                                                                        sort_col.set(Some(i));
                                                                                        sort_dir.set(SortDir::Asc);
                                                                                    }
                                                                                }
                                                                            >
                                                                                <span class="truncate">{header}</span>
                                                                                <span
                                                                                    class="material-symbols-outlined csv-sort-icon"
                                                                                    style="font-size:14px;"
                                                                                >
                                                                                    {move || {
                                                                                        if sort_col.get() == Some(i) {
                                                                                            match sort_dir.get() {
                                                                                                SortDir::Asc => "arrow_upward",
                                                                                                SortDir::Desc => "arrow_downward",
                                                                                            }
                                                                                        } else {
                                                                                            "unfold_more"
                                                                                        }
                                                                                    }}
                                                                                </span>
                                                                            </div>
                                                                            // Column resize handle.
                                                                            <div
                                                                                class="csv-resize-handle"
                                                                                on:mousedown=move |e: web_sys::MouseEvent| {
                                                                                    e.prevent_default();
                                                                                    e.stop_propagation();
                                                                                    resize_col.set(Some(i));
                                                                                    resize_x0.set(e.client_x() as f64);
                                                                                    let w = col_widths.get_untracked()
                                                                                        .get(i)
                                                                                        .copied()
                                                                                        .unwrap_or(default_w);
                                                                                    resize_w0.set(w);
                                                                                }
                                                                            />
                                                                        </th>
                                                                    }
                                                                })
                                                                .collect::<Vec<_>>()}
                                                        </tr>
                                                    </thead>
                                                    <tbody>
                                                        {move || {
                                                            let mut sorted = rows_data.clone();
                                                            if let Some(ci) = sort_col.get() {
                                                                let dir = sort_dir.get();
                                                                sorted.sort_by(|a, b| {
                                                                    let va = a.get(ci).map(String::as_str).unwrap_or("");
                                                                    let vb = b.get(ci).map(String::as_str).unwrap_or("");
                                                                    // Try numeric comparison first.
                                                                    let cmp = match (va.parse::<f64>(), vb.parse::<f64>()) {
                                                                        (Ok(na), Ok(nb)) => na.partial_cmp(&nb).unwrap_or(std::cmp::Ordering::Equal),
                                                                        _ => va.to_ascii_lowercase().cmp(&vb.to_ascii_lowercase()),
                                                                    };
                                                                    match dir {
                                                                        SortDir::Asc => cmp,
                                                                        SortDir::Desc => cmp.reverse(),
                                                                    }
                                                                });
                                                            }
                                                            sorted
                                                                .into_iter()
                                                                .map(|row| {
                                                                    view! {
                                                                        <tr>
                                                                            {row
                                                                                .into_iter()
                                                                                .map(|cell| {
                                                                                    view! { <td><span class="truncate block">{cell}</span></td> }
                                                                                })
                                                                                .collect::<Vec<_>>()}
                                                                        </tr>
                                                                    }
                                                                })
                                                                .collect::<Vec<_>>()
                                                        }}
                                                    </tbody>
                                                </table>
                                            </div>

                                            // Footer — row/column count.
                                            <div class="flex-shrink-0 border-t border-gray-200 dark:border-gray-700 \
                                                        bg-white dark:bg-gray-800 px-3 py-1.5 text-xs text-gray-400 \
                                                        text-center">
                                                {format!("{row_count} rows \u{00d7} {col_count} columns")}
                                            </div>
                                        </div>
                                    }
                                    .into_any()
                                }
                            }
                        }
                    }
                }

                // ── Unsupported ───────────────────────────────────────────────
                PreviewKind::Unsupported => view! {
                    <div class="flex-1 flex flex-col items-center justify-center \
                                p-6 text-center text-gray-400">
                        <span class="material-symbols-outlined"
                            style="font-size:40px; display:block; margin-bottom:8px;">
                            "visibility_off"
                        </span>
                        <p class="text-sm">"Preview not available for this file type."</p>
                        <p class="text-xs mt-1 text-gray-300">
                            {target.content_type.clone()}
                        </p>
                    </div>
                }
                .into_any(),

                // ── Text and Markdown ─────────────────────────────────────────
                PreviewKind::Text | PreviewKind::Markdown => {
                    match text_resource.map(|r| r.clone()) {
                        // Still loading.
                        None | Some(None) => view! {
                            <div class="flex-1 flex items-center justify-center">
                                <span class="material-symbols-outlined \
                                             text-gray-300 animate-spin"
                                    style="font-size:32px;">
                                    "progress_activity"
                                </span>
                            </div>
                        }
                        .into_any(),

                        // File too large.
                        Some(Some(Err(UiError::FileTooLarge(_)))) => view! {
                            <div class="flex-1 flex flex-col items-center \
                                        justify-center p-6 text-center text-gray-400">
                                <span class="material-symbols-outlined"
                                    style="font-size:40px; display:block; margin-bottom:8px;">
                                    "data_usage"
                                </span>
                                <p class="text-sm">"File exceeds the 1 MiB preview limit."</p>
                                <p class="text-xs mt-1 text-gray-300">
                                    "Use the Download button to save and open locally."
                                </p>
                            </div>
                        }
                        .into_any(),

                        // Fetch error.
                        Some(Some(Err(ref e))) => {
                            let msg = e.to_string();
                            view! {
                                <div class="flex-1 flex items-center justify-center \
                                            p-4 text-center text-red-500 text-sm">
                                    {msg}
                                </div>
                            }
                            .into_any()
                        }

                        // Content ready.
                        Some(Some(Ok(ref text))) => {
                            if kind == PreviewKind::Markdown {
                                // innerHTML is set by the Effect above via md_ref.
                                view! {
                                    <div class="flex-1 overflow-auto p-4">
                                        <div
                                            node_ref=md_ref
                                            class="prose prose-sm max-w-none \
                                                   text-gray-800 dark:text-gray-200 leading-relaxed"
                                        />
                                    </div>
                                }
                                .into_any()
                            } else {
                                let text = text.clone();
                                view! {
                                    <div class="flex-1 overflow-auto p-4 bg-gray-50">
                                        <pre class="text-xs font-mono text-gray-700 \
                                                    whitespace-pre-wrap break-words \
                                                    leading-relaxed">
                                            {text}
                                        </pre>
                                    </div>
                                }
                                .into_any()
                            }
                        }
                    }
                }
            }
        }}
    }
}
