use common::{
    CatalogPath,
    dto::{PatchFileRequest, PatchFolderRequest},
};
use leptos::prelude::*;

use crate::api;
use crate::app::{ContentsResource, ItemKind, ModalState, SelectedItem};

#[component]
pub fn RenameModal(path: CatalogPath, current_name: String, kind: ItemKind) -> impl IntoView {
    let modal = use_context::<RwSignal<Option<ModalState>>>().expect("modal context missing");
    let selected = use_context::<RwSignal<Vec<SelectedItem>>>().expect("selected context missing");
    let contents = use_context::<ContentsResource>().expect("contents context missing");
    let error_msg = use_context::<RwSignal<Option<String>>>().expect("error_msg context missing");
    let catalog_version = use_context::<RwSignal<u32>>().expect("catalog_version context missing");

    let new_name = RwSignal::new(current_name.clone());
    let submitting = RwSignal::new(false);

    let path_clone = path.clone();
    let kind_clone = kind.clone();

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let nm = new_name.get_untracked().trim().to_owned();
        if nm.is_empty() || nm == current_name {
            modal.set(None);
            return;
        }
        submitting.set(true);
        let p = path_clone.clone();
        let k = kind_clone.clone();

        wasm_bindgen_futures::spawn_local(async move {
            let result = match k {
                ItemKind::Folder => api::patch_folder(
                    p,
                    PatchFolderRequest {
                        name: Some(nm),
                        new_parent_path: None,
                    },
                )
                .await
                .map(|_| ()),
                ItemKind::File => api::patch_file(
                    p,
                    PatchFileRequest {
                        name: Some(nm),
                        new_folder_path: None,
                    },
                )
                .await
                .map(|_| ()),
            };
            match result {
                Ok(_) => {
                    selected.set(Vec::new());
                    catalog_version.update(|v| *v += 1);
                    modal.set(None);
                    contents.refetch();
                }
                Err(e) => {
                    error_msg.set(Some(e.to_string()));
                    submitting.set(false);
                }
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
                    <span class="material-symbols-outlined text-gray-500">
                        "drive_file_rename_outline"
                    </span>
                    <h2 class="text-sm font-semibold text-gray-900">"Rename"</h2>
                </div>
                <form on:submit=on_submit>
                    <input
                        type="text"
                        class="w-full border border-gray-300 rounded px-3 py-2 text-sm \
                               focus:outline-none focus:ring-2 focus:ring-gray-400 mb-4"
                        prop:value=move || new_name.get()
                        on:input=move |ev| new_name.set(event_target_value(&ev))
                        autofocus
                    />
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
                            type="submit"
                            class="px-3 py-1.5 text-sm font-medium bg-gray-900 text-white \
                                   rounded hover:bg-gray-700 disabled:opacity-40 transition-colors"
                            prop:disabled=move || submitting.get()
                        >
                            {move || if submitting.get() { "Renaming…" } else { "Rename" }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}
