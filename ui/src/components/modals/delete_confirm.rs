use leptos::prelude::*;

use crate::api;
use crate::app::{ContentsResource, ItemKind, ModalState, SelectedItem};

#[component]
pub fn DeleteConfirmModal(items: Vec<SelectedItem>, file_count: usize) -> impl IntoView {
    let modal = use_context::<RwSignal<Option<ModalState>>>().expect("modal context missing");
    let selected = use_context::<RwSignal<Vec<SelectedItem>>>().expect("selected context missing");
    let contents = use_context::<ContentsResource>().expect("contents context missing");
    let error_msg = use_context::<RwSignal<Option<String>>>().expect("error_msg context missing");
    let catalog_version = use_context::<RwSignal<u32>>().expect("catalog_version context missing");

    let submitting = RwSignal::new(false);

    // Build a human-readable summary of what will be deleted.
    let folder_count = items.iter().filter(|i| i.kind == ItemKind::Folder).count();
    let direct_file_count = items.iter().filter(|i| i.kind == ItemKind::File).count();

    let desc = {
        let mut parts: Vec<String> = Vec::new();
        if folder_count > 0 {
            parts.push(format!(
                "{} folder{}",
                folder_count,
                if folder_count == 1 { "" } else { "s" }
            ));
        }
        if direct_file_count > 0 {
            parts.push(format!(
                "{} file{}",
                direct_file_count,
                if direct_file_count == 1 { "" } else { "s" }
            ));
        }
        parts.join(" and ")
    };

    let nested_msg = if file_count > 0 {
        Some(format!(
            "This will also permanently remove {} file{} contained within.",
            file_count,
            if file_count == 1 { "" } else { "s" }
        ))
    } else {
        None
    };

    let items_for_confirm = items.clone();
    let on_confirm = move |_| {
        let mv_items = items_for_confirm.clone();
        submitting.set(true);

        wasm_bindgen_futures::spawn_local(async move {
            let mut had_error = false;
            for item in mv_items {
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
            submitting.set(false);
            if !had_error {
                selected.set(Vec::new());
                catalog_version.update(|v| *v += 1);
                modal.set(None);
                contents.refetch();
            }
        });
    };

    view! {
        <div
            class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
            on:click=move |_| modal.set(None)
        >
            <div
                class="bg-white rounded-lg shadow-xl p-6 w-full max-w-sm mx-4"
                on:click=|ev| ev.stop_propagation()
            >
                // Header
                <div class="flex items-center gap-2 mb-4">
                    <span class="material-symbols-outlined text-red-500">"delete_forever"</span>
                    <h2 class="text-sm font-semibold text-gray-900">"Confirm Delete"</h2>
                </div>

                <p class="text-sm text-gray-600 mb-3">
                    "You are about to permanently delete "
                    <strong>{desc}</strong>
                    "."
                </p>

                {nested_msg.map(|msg| view! {
                    <div class="flex items-start gap-2 p-2 bg-red-50 border border-red-200 \
                                rounded text-sm text-red-700 mb-4">
                        <span class="material-symbols-outlined text-red-500 flex-shrink-0"
                            style="font-size:16px; margin-top:1px;">"warning"</span>
                        {msg}
                    </div>
                })}

                <div class="flex gap-2 justify-end">
                    <button
                        type="button"
                        class="px-3 py-1.5 text-sm text-gray-500 hover:text-gray-900 \
                               transition-colors"
                        on:click=move |_| modal.set(None)
                    >
                        "Cancel"
                    </button>
                    <button
                        type="button"
                        class="px-3 py-1.5 text-sm font-medium bg-red-600 text-white \
                               rounded hover:bg-red-700 disabled:opacity-40 transition-colors"
                        prop:disabled=move || submitting.get()
                        on:click=on_confirm
                    >
                        {move || if submitting.get() { "Deleting…" } else { "Delete" }}
                    </button>
                </div>
            </div>
        </div>
    }
}
