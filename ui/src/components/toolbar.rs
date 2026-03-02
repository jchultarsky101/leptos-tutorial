use common::CatalogPath;
use leptos::prelude::*;

use crate::api;
use crate::app::{ContentsResource, ItemKind, ModalState, SelectedItem};

#[component]
pub fn Toolbar() -> impl IntoView {
    let current_path =
        use_context::<RwSignal<CatalogPath>>().expect("current_path context missing");
    let selected = use_context::<RwSignal<Vec<SelectedItem>>>().expect("selected context missing");
    let modal = use_context::<RwSignal<Option<ModalState>>>().expect("modal context missing");
    let error_msg = use_context::<RwSignal<Option<String>>>().expect("error_msg context missing");
    let contents = use_context::<ContentsResource>().expect("contents context missing");

    let has_selection = move || !selected.get().is_empty();
    let single_selection = move || selected.get().len() == 1;

    // Delete all selected items.
    let on_delete = move |_| {
        let items = selected.get_untracked();
        if items.is_empty() {
            return;
        }
        wasm_bindgen_futures::spawn_local(async move {
            let mut had_error = false;
            for item in items {
                let result = match item.kind {
                    ItemKind::Folder => api::delete_folder(item.path).await,
                    ItemKind::File => api::delete_file(item.path).await,
                };
                if let Err(e) = result {
                    error_msg.set(Some(e.to_string()));
                    had_error = true;
                    break;
                }
            }
            if !had_error {
                selected.set(Vec::new());
                contents.refetch();
            }
        });
    };

    let on_rename = move |_| {
        let items = selected.get_untracked();
        if let Some(item) = items.into_iter().next() {
            let current_name = item.path.name().to_owned();
            modal.set(Some(ModalState::Rename {
                path: item.path,
                current_name,
                kind: item.kind,
            }));
        }
    };

    let on_move = move |_| {
        let items = selected.get_untracked();
        if !items.is_empty() {
            modal.set(Some(ModalState::Move { items }));
        }
    };

    view! {
        <div class="flex items-center gap-2 mb-4 flex-wrap">
            // Always-visible buttons
            <button
                class="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium bg-blue-600 text-white rounded-md hover:bg-blue-700 focus:outline-none"
                on:click=move |_| modal.set(Some(ModalState::CreateFolder))
            >
                <span>"➕"</span>
                "New Folder"
            </button>
            <button
                class="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium bg-white border border-gray-300 text-gray-700 rounded-md hover:bg-gray-50 focus:outline-none"
                on:click=move |_| {
                    modal.set(Some(ModalState::Upload {
                        folder_path: current_path.get_untracked(),
                    }))
                }
            >
                <span>"⬆"</span>
                "Upload"
            </button>

            // Selection-conditional buttons
            <Show when=single_selection>
                <button
                    class="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium bg-white border border-gray-300 text-gray-700 rounded-md hover:bg-gray-50 focus:outline-none"
                    on:click=on_rename
                >
                    <span>"✏"</span>
                    "Rename"
                </button>
            </Show>
            <Show when=has_selection>
                <button
                    class="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium bg-white border border-gray-300 text-gray-700 rounded-md hover:bg-gray-50 focus:outline-none"
                    on:click=on_move
                >
                    <span>"📦"</span>
                    "Move"
                </button>
                <button
                    class="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium bg-red-600 text-white rounded-md hover:bg-red-700 focus:outline-none"
                    on:click=on_delete
                >
                    <span>"🗑"</span>
                    "Delete"
                </button>
            </Show>

            // Selection count badge
            <Show when=has_selection>
                <span class="ml-2 text-sm text-gray-500">
                    {move || format!("{} selected", selected.get().len())}
                </span>
            </Show>
        </div>
    }
}
