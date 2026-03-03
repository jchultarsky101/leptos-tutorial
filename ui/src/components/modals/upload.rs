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
            class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
            on:click=move |_| modal.set(None)
        >
            <div
                class="bg-white rounded-lg shadow-xl p-6 w-full max-w-md mx-4"
                on:click=|ev| ev.stop_propagation()
            >
                // Header
                <div class="flex items-center gap-2 mb-1">
                    <span class="material-symbols-outlined text-gray-500">"upload"</span>
                    <h2 class="text-sm font-semibold text-gray-900">"Upload Files"</h2>
                </div>
                <p class="text-xs text-gray-400 mb-4 ml-7">
                    "Into: "
                    <code class="font-mono bg-gray-100 px-1 rounded">
                        {folder_for_display.as_str().to_owned()}
                    </code>
                </p>

                <form on:submit=on_submit>
                    <input
                        type="file"
                        multiple
                        node_ref=input_ref
                        class="block w-full text-sm text-gray-500 mb-4 \
                               file:mr-3 file:py-1.5 file:px-3 file:rounded \
                               file:border file:border-gray-300 \
                               file:text-xs file:font-medium file:text-gray-700 \
                               file:bg-white hover:file:bg-gray-50 \
                               file:transition-colors file:cursor-pointer"
                    />

                    <Show when=move || !status.get().is_empty()>
                        <div class="flex items-start gap-2 mb-3 p-2 bg-amber-50 \
                                    border border-amber-200 rounded text-sm text-amber-700">
                            <span class="material-symbols-outlined text-amber-500"
                                style="font-size:16px; margin-top:1px;">
                                "warning"
                            </span>
                            {move || status.get()}
                        </div>
                    </Show>

                    <Show when=move || !conflicts.get().is_empty()>
                        <ul class="text-xs text-gray-500 mb-3 list-disc list-inside space-y-0.5">
                            {move || conflicts.get().iter().map(|f| {
                                view! { <li>{f.name()}</li> }
                            }).collect_view()}
                        </ul>
                    </Show>

                    <div class="flex gap-2 justify-end flex-wrap">
                        <button
                            type="button"
                            class="px-3 py-1.5 text-sm text-gray-500 hover:text-gray-900 \
                                   transition-colors"
                            on:click=move |_| modal.set(None)
                        >
                            "Cancel"
                        </button>
                        <Show when=move || !conflicts.get().is_empty()>
                            <button
                                type="button"
                                class="px-3 py-1.5 text-sm font-medium bg-amber-500 text-white \
                                       rounded hover:bg-amber-600 disabled:opacity-40 transition-colors"
                                prop:disabled=move || uploading.get()
                                on:click=on_override
                            >
                                "Override Existing"
                            </button>
                        </Show>
                        <button
                            type="submit"
                            class="px-3 py-1.5 text-sm font-medium bg-gray-900 text-white \
                                   rounded hover:bg-gray-700 disabled:opacity-40 transition-colors"
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
