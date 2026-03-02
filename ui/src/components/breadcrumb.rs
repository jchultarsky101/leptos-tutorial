use common::CatalogPath;
use leptos::prelude::*;

use crate::app::SelectedItem;

#[component]
pub fn Breadcrumb() -> impl IntoView {
    let current_path =
        use_context::<RwSignal<CatalogPath>>().expect("current_path context missing");
    let selected = use_context::<RwSignal<Vec<SelectedItem>>>().expect("selected context missing");

    view! {
        <nav class="flex items-center flex-wrap gap-1 text-sm text-gray-600 mb-3 py-2">
            {move || {
                let path = current_path.get();
                let path_str = path.as_str().to_owned();

                // Build list of (display_name, path_string) segments.
                let mut segs: Vec<(String, String)> =
                    vec![("Root".into(), "/".into())];
                if !path.is_root() {
                    let mut cum = String::new();
                    for part in path_str.trim_start_matches('/').split('/') {
                        cum.push('/');
                        cum.push_str(part);
                        segs.push((part.to_owned(), cum.clone()));
                    }
                }

                let last_idx = segs.len() - 1;
                segs.into_iter()
                    .enumerate()
                    .map(move |(i, (name, p_str))| {
                        let is_last = i == last_idx;
                        let sep = if i > 0 {
                            view! { <span class="text-gray-400 select-none">" / "</span> }
                                .into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        };

                        let node = if is_last {
                            view! {
                                <span class="font-semibold text-gray-900">{name}</span>
                            }
                            .into_any()
                        } else {
                            view! {
                                <button
                                    class="hover:text-blue-600 hover:underline"
                                    on:click=move |_| {
                                        if let Ok(p) = CatalogPath::new(&p_str) {
                                            selected.set(Vec::new());
                                            current_path.set(p);
                                        }
                                    }
                                >
                                    {name}
                                </button>
                            }
                            .into_any()
                        };

                        view! {
                            {sep}
                            {node}
                        }
                    })
                    .collect_view()
            }}
        </nav>
    }
}
