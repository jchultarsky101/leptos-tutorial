use common::{CatalogPath, dto::SearchResultsDto};
use leptos::prelude::*;

use super::search_bar::clear_search_input;
use crate::app::PreviewTarget;

/// Displays search results, replacing the normal file grid when a search is active.
#[component]
pub fn SearchResults() -> impl IntoView {
    let search_results: RwSignal<Option<SearchResultsDto>> =
        use_context().expect("search_results context");
    let search_query: RwSignal<Option<String>> = use_context().expect("search_query context");
    let current_path: RwSignal<CatalogPath> = use_context().expect("current_path context");
    let preview_file: RwSignal<Option<PreviewTarget>> =
        use_context().expect("preview_file context");

    let clear = move |_| {
        search_query.set(None);
        clear_search_input();
    };

    view! {
        <div class="flex-1 min-h-0 flex flex-col bg-white dark:bg-gray-800 \
                     border border-gray-200 dark:border-gray-700 \
                     rounded-lg shadow-sm overflow-hidden">
            {move || {
                let data = search_results.get();
                match data {
                    None => view! {
                        <div class="flex-1 flex items-center justify-center text-sm text-gray-400 dark:text-gray-500">
                            "Searching..."
                        </div>
                    }.into_any(),
                    Some(dto) => {
                        let count = dto.results.len();
                        let query_display = dto.query.clone();
                        let results = dto.results.clone();

                        view! {
                            <div class="flex flex-col flex-1 min-h-0">
                                // Header
                                <div class="flex-shrink-0 px-4 py-2 border-b border-gray-100 dark:border-gray-700 \
                                             flex items-center justify-between">
                                    <span class="text-sm text-gray-600 dark:text-gray-400">
                                        {format!("{count} result{} for ", if count == 1 { "" } else { "s" })}
                                        <span class="font-medium text-gray-800 dark:text-gray-200">
                                            {format!("\"{query_display}\"")}
                                        </span>
                                    </span>
                                    <button
                                        class="text-xs text-blue-500 hover:text-blue-700 \
                                               focus:outline-none"
                                        on:click=clear
                                    >
                                        "Clear search"
                                    </button>
                                </div>

                                // Results list
                                <div class="flex-1 overflow-y-auto">
                                    {if results.is_empty() {
                                        view! {
                                            <div class="flex items-center justify-center \
                                                        h-32 text-sm text-gray-400 dark:text-gray-500">
                                                "No results found."
                                            </div>
                                        }.into_any()
                                    } else {
                                        let items = results.into_iter().map(|r| {
                                            let path = r.path.clone();
                                            let name = r.name.clone();
                                            let kind = r.kind.clone();
                                            let content_type = r.content_type.clone();
                                            let match_source = r.match_source.clone();
                                            let snippet = r.snippet.clone();
                                            let size_bytes = r.size_bytes;
                                            let path_str = r.path.as_str().to_owned();

                                            let on_click = {
                                                let path = path.clone();
                                                let kind = kind.clone();
                                                let content_type = content_type.clone();
                                                move |_| {
                                                    if kind == "folder" {
                                                        current_path.set(path.clone());
                                                    } else {
                                                        if let Some(parent) = path.parent() {
                                                            current_path.set(parent);
                                                        }
                                                        let ct = content_type
                                                            .clone()
                                                            .unwrap_or_else(|| {
                                                                "application/octet-stream".into()
                                                            });
                                                        preview_file.set(Some(PreviewTarget {
                                                            path: path.clone(),
                                                            content_type: ct,
                                                        }));
                                                    }
                                                    search_query.set(None);
                                                    clear_search_input();
                                                }
                                            };

                                            let icon = if kind == "folder" {
                                                "folder"
                                            } else {
                                                "description"
                                            };

                                            let badge_class = match match_source.as_str() {
                                                "name" => "bg-blue-100 text-blue-700",
                                                "content" => "bg-green-100 text-green-700",
                                                "both" => "bg-purple-100 text-purple-700",
                                                _ => "bg-gray-100 text-gray-600",
                                            };

                                            view! {
                                                <button
                                                    class="w-full text-left px-4 py-2.5 \
                                                           hover:bg-blue-50 dark:hover:bg-gray-700 \
                                                           border-b border-gray-50 dark:border-gray-700 \
                                                           flex items-start gap-3 focus:outline-none \
                                                           transition-colors"
                                                    on:click=on_click
                                                >
                                                    <span class="material-symbols-outlined \
                                                                 text-gray-400 mt-0.5"
                                                          style="font-size:20px;">
                                                        {icon}
                                                    </span>
                                                    <div class="flex-1 min-w-0">
                                                        <div class="flex items-center gap-2">
                                                            <span class="text-sm font-medium \
                                                                         text-gray-800 dark:text-gray-100 truncate">
                                                                {name}
                                                            </span>
                                                            <span class={format!(
                                                                "text-[10px] px-1.5 py-0.5 \
                                                                 rounded font-medium {badge_class}"
                                                            )}>
                                                                {match_source}
                                                            </span>
                                                            {size_bytes.map(|s| {
                                                                let display = if s < 1024 {
                                                                    format!("{s} B")
                                                                } else if s < 1_048_576 {
                                                                    format!("{:.1} KB", s as f64 / 1024.0)
                                                                } else {
                                                                    format!("{:.1} MB", s as f64 / 1_048_576.0)
                                                                };
                                                                view! {
                                                                    <span class="text-[10px] \
                                                                                 text-gray-400">
                                                                        {display}
                                                                    </span>
                                                                }
                                                            })}
                                                        </div>
                                                        <div class="text-xs text-gray-400 dark:text-gray-500 \
                                                                     truncate mt-0.5">
                                                            {path_str}
                                                        </div>
                                                        {snippet.map(|s| view! {
                                                            <div class="text-xs text-gray-500 dark:text-gray-400 \
                                                                        mt-1 font-mono \
                                                                        bg-gray-50 dark:bg-gray-700 rounded \
                                                                        px-2 py-1 truncate">
                                                                {s}
                                                            </div>
                                                        })}
                                                    </div>
                                                </button>
                                            }
                                        }).collect::<Vec<_>>();
                                        view! { <div>{items}</div> }.into_any()
                                    }}
                                </div>
                            </div>
                        }.into_any()
                    }
                }
            }}
        </div>
    }
}
