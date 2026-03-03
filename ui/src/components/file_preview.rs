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
    Unsupported,
}

fn classify(content_type: &str, filename: &str) -> PreviewKind {
    if content_type.starts_with("image/") {
        return PreviewKind::Image;
    }
    if content_type == "text/markdown"
        || content_type == "text/x-markdown"
        || filename.ends_with(".md")
        || filename.ends_with(".markdown")
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
    // Returns None for image/unsupported (no fetch needed) or while target is None.
    let text_resource: LocalResource<Option<Result<String, UiError>>> =
        LocalResource::new(move || {
            let target = preview_file.get();
            async move {
                let t = target?;
                let kind = classify(&t.content_type, t.path.name());
                match kind {
                    PreviewKind::Text | PreviewKind::Markdown => {
                        Some(api::fetch_file_content(&t.path).await)
                    }
                    PreviewKind::Image | PreviewKind::Unsupported => None,
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
                                                   text-gray-800 leading-relaxed"
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
