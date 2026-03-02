use common::CatalogPath;
use leptos::prelude::*;

use crate::api;
use crate::app::{ContentsResource, ModalState};

#[component]
pub fn CreateFolderModal() -> impl IntoView {
    let modal = use_context::<RwSignal<Option<ModalState>>>().expect("modal context missing");
    let current_path =
        use_context::<RwSignal<CatalogPath>>().expect("current_path context missing");
    let contents = use_context::<ContentsResource>().expect("contents context missing");
    let error_msg = use_context::<RwSignal<Option<String>>>().expect("error_msg context missing");

    let name = RwSignal::new(String::new());
    let submitting = RwSignal::new(false);

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let folder_name = name.get_untracked().trim().to_owned();
        if folder_name.is_empty() {
            return;
        }
        let parent = current_path.get_untracked();
        submitting.set(true);

        wasm_bindgen_futures::spawn_local(async move {
            match api::create_folder(parent, &folder_name).await {
                Ok(_) => {
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
            class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50"
            on:click=move |_| modal.set(None)
        >
            <div
                class="bg-white rounded-lg shadow-xl p-6 w-full max-w-sm mx-4"
                on:click=|ev| ev.stop_propagation()
            >
                <h2 class="text-lg font-semibold text-gray-900 mb-4">"New Folder"</h2>
                <form on:submit=on_submit>
                    <input
                        type="text"
                        placeholder="Folder name"
                        class="w-full border border-gray-300 rounded-md px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 mb-4"
                        prop:value=move || name.get()
                        on:input=move |ev| name.set(event_target_value(&ev))
                        autofocus
                    />
                    <div class="flex gap-2 justify-end">
                        <button
                            type="button"
                            class="px-4 py-2 text-sm text-gray-600 hover:text-gray-900"
                            on:click=move |_| modal.set(None)
                        >
                            "Cancel"
                        </button>
                        <button
                            type="submit"
                            class="px-4 py-2 text-sm font-medium bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
                            prop:disabled=move || submitting.get()
                        >
                            {move || if submitting.get() { "Creating…" } else { "Create" }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}
