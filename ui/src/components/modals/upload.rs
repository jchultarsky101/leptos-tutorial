use common::CatalogPath;
use leptos::prelude::*;

use crate::api;
use crate::app::{ContentsResource, ModalState};

/// Bundles the signal handles needed by the upload logic.
#[derive(Copy, Clone)]
struct UploadCtx {
    uploading: RwSignal<bool>,
    status: RwSignal<String>,
    conflicts: RwSignal<Vec<web_sys::File>>,
    modal: RwSignal<Option<ModalState>>,
    contents: ContentsResource,
    error_msg: RwSignal<Option<String>>,
}

/// Perform the actual upload loop.
fn run_upload(files: Vec<web_sys::File>, overwrite: bool, folder: CatalogPath, ctx: UploadCtx) {
    ctx.uploading.set(true);
    ctx.status.set(String::new());
    ctx.conflicts.set(Vec::new());

    wasm_bindgen_futures::spawn_local(async move {
        let mut new_conflicts = Vec::new();
        let mut had_other_error = false;

        for file in files {
            let name = file.name();
            match api::upload_file(&folder, &name, file.clone(), overwrite).await {
                Ok(_) => {}
                Err(crate::error::UiError::Api { status: 409, .. }) if !overwrite => {
                    new_conflicts.push(file);
                }
                Err(e) => {
                    ctx.error_msg.set(Some(e.to_string()));
                    had_other_error = true;
                    break;
                }
            }
        }

        ctx.uploading.set(false);
        if !had_other_error {
            if new_conflicts.is_empty() {
                ctx.modal.set(None);
                ctx.contents.refetch();
            } else {
                ctx.conflicts.set(new_conflicts);
                ctx.status
                    .set("Some files already exist. Click Override to replace them.".into());
            }
        }
    });
}

#[component]
pub fn UploadModal(folder_path: CatalogPath) -> impl IntoView {
    let modal = use_context::<RwSignal<Option<ModalState>>>().expect("modal context missing");
    let contents = use_context::<ContentsResource>().expect("contents context missing");
    let error_msg = use_context::<RwSignal<Option<String>>>().expect("error_msg context missing");

    let input_ref = NodeRef::<leptos::html::Input>::new();
    // Files that already exist (HTTP 409).
    let conflicts: RwSignal<Vec<web_sys::File>> = RwSignal::new(Vec::new());
    let uploading = RwSignal::new(false);
    let status = RwSignal::new(String::new());

    let ctx = UploadCtx {
        uploading,
        status,
        conflicts,
        modal,
        contents,
        error_msg,
    };

    let folder_for_submit = folder_path.clone();
    let folder_for_display = folder_path.clone();
    let folder_stored = StoredValue::new(folder_path);

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Some(input) = input_ref.get_untracked() else {
            return;
        };
        let Some(file_list) = input.files() else {
            return;
        };
        let mut files = Vec::new();
        for i in 0..file_list.length() {
            if let Some(f) = file_list.item(i) {
                files.push(f);
            }
        }
        if files.is_empty() {
            return;
        }
        run_upload(files, false, folder_for_submit.clone(), ctx);
    };

    let on_override = move |_| {
        let files = conflicts.get_untracked();
        if files.is_empty() {
            return;
        }
        run_upload(files, true, folder_stored.get_value(), ctx);
    };

    view! {
        <div
            class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50"
            on:click=move |_| modal.set(None)
        >
            <div
                class="bg-white rounded-lg shadow-xl p-6 w-full max-w-md mx-4"
                on:click=|ev| ev.stop_propagation()
            >
                <h2 class="text-lg font-semibold text-gray-900 mb-1">"Upload Files"</h2>
                <p class="text-sm text-gray-500 mb-4">
                    "Uploading into: "
                    <code class="font-mono text-xs bg-gray-100 px-1 rounded">
                        {folder_for_display.as_str().to_owned()}
                    </code>
                </p>

                <form on:submit=on_submit>
                    <input
                        type="file"
                        multiple
                        node_ref=input_ref
                        class="block w-full text-sm text-gray-600 file:mr-4 file:py-2 file:px-4 file:rounded-md file:border-0 file:text-sm file:font-medium file:bg-blue-50 file:text-blue-700 hover:file:bg-blue-100 mb-4"
                    />

                    <Show when=move || !status.get().is_empty()>
                        <p class="text-sm text-amber-600 mb-3">{move || status.get()}</p>
                    </Show>

                    <Show when=move || !conflicts.get().is_empty()>
                        <ul class="text-sm text-gray-600 mb-3 list-disc list-inside">
                            {move || conflicts.get().iter().map(|f| {
                                view! { <li>{f.name()}</li> }
                            }).collect_view()}
                        </ul>
                    </Show>

                    <div class="flex gap-2 justify-end flex-wrap">
                        <button
                            type="button"
                            class="px-4 py-2 text-sm text-gray-600 hover:text-gray-900"
                            on:click=move |_| modal.set(None)
                        >
                            "Cancel"
                        </button>
                        <Show when=move || !conflicts.get().is_empty()>
                            <button
                                type="button"
                                class="px-4 py-2 text-sm font-medium bg-amber-500 text-white rounded-md hover:bg-amber-600 disabled:opacity-50"
                                prop:disabled=move || uploading.get()
                                on:click=on_override
                            >
                                "Override Existing"
                            </button>
                        </Show>
                        <button
                            type="submit"
                            class="px-4 py-2 text-sm font-medium bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
                            prop:disabled=move || uploading.get()
                        >
                            {move || if uploading.get() { "Uploading…" } else { "Upload" }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}
