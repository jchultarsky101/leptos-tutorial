use common::CatalogPath;
use leptos::prelude::*;
use wasm_bindgen::JsCast as _;

use crate::api;
use crate::app::{ContentsResource, ItemKind, ModalState, SelectedItem};

// ── Icon-only toolbar button with hover tooltip ────────────────────────────

#[component]
fn ToolbarBtn(
    icon: &'static str,
    label: &'static str,
    #[prop(optional)] danger: bool,
    on_click: impl Fn() + 'static,
) -> impl IntoView {
    let btn_class = if danger {
        "w-9 h-9 inline-flex items-center justify-center rounded \
         border border-red-200 text-red-500 \
         hover:bg-red-50 focus:outline-none transition-colors"
    } else {
        "w-9 h-9 inline-flex items-center justify-center rounded \
         border border-gray-300 text-gray-600 \
         hover:bg-gray-100 focus:outline-none transition-colors"
    };

    view! {
        <div class="group relative">
            <button class=btn_class on:click=move |_| on_click()>
                <span class="material-symbols-outlined" style="font-size:18px;">{icon}</span>
            </button>
            // Tooltip
            <span class="pointer-events-none absolute bottom-full left-1/2 -translate-x-1/2 \
                         mb-1.5 whitespace-nowrap rounded px-2 py-1 \
                         text-xs font-medium bg-gray-900 text-white \
                         opacity-0 group-hover:opacity-100 transition-opacity z-50">
                {label}
            </span>
        </div>
    }
}

// ── Toolbar ────────────────────────────────────────────────────────────────

#[component]
pub fn Toolbar() -> impl IntoView {
    let current_path =
        use_context::<RwSignal<CatalogPath>>().expect("current_path context missing");
    let selected = use_context::<RwSignal<Vec<SelectedItem>>>().expect("selected context missing");
    let modal = use_context::<RwSignal<Option<ModalState>>>().expect("modal context missing");
    let error_msg = use_context::<RwSignal<Option<String>>>().expect("error_msg context missing");
    let contents = use_context::<ContentsResource>().expect("contents context missing");
    let catalog_version = use_context::<RwSignal<u32>>().expect("catalog_version context missing");

    let has_selection = move || !selected.get().is_empty();
    let single_selection = move || selected.get().len() == 1;
    let single_file_selected = move || {
        let sel = selected.get();
        sel.len() == 1 && sel.first().is_some_and(|i| i.kind == ItemKind::File)
    };

    // ── Download ──────────────────────────────────────────────────────────
    // Creates a temporary <a download> in the DOM and clicks it so the browser
    // triggers a Save-As dialog without navigating.
    let on_download = move || {
        let items = selected.get_untracked();
        let Some(item) = items.into_iter().find(|i| i.kind == ItemKind::File) else {
            return;
        };
        let Some(win) = web_sys::window() else { return };
        let Some(doc) = win.document() else { return };
        let Ok(el) = doc.create_element("a") else {
            return;
        };
        let Ok(anchor) = el.dyn_into::<web_sys::HtmlAnchorElement>() else {
            return;
        };

        let href = format!(
            "http://localhost:3000/files/{}",
            item.path.as_str().trim_start_matches('/')
        );
        anchor.set_href(&href);
        anchor.set_download(item.path.name());

        // Must be in DOM for Firefox.
        if let Some(body) = doc.body() {
            let _ = body.append_child(&anchor);
            anchor.click();
            let _ = body.remove_child(&anchor);
        } else {
            anchor.click();
        }
    };

    // ── Delete ────────────────────────────────────────────────────────────
    // Files → immediate delete.
    // Any folder in selection → count contained files recursively, then show
    // the DeleteConfirm modal so the user sees what they are about to remove.
    let on_delete = move || {
        let items = selected.get_untracked();
        if items.is_empty() {
            return;
        }

        let has_folders = items.iter().any(|i| i.kind == ItemKind::Folder);

        if !has_folders {
            // Only files selected — delete immediately, no confirm needed.
            wasm_bindgen_futures::spawn_local(async move {
                let mut had_error = false;
                for item in items {
                    if let Err(e) = api::delete_file(item.path).await {
                        error_msg.set(Some(e.to_string()));
                        had_error = true;
                        break;
                    }
                }
                if !had_error {
                    selected.set(Vec::new());
                    catalog_version.update(|v| *v += 1);
                    contents.refetch();
                }
            });
        } else {
            // Count all files nested inside selected folders, then show modal.
            wasm_bindgen_futures::spawn_local(async move {
                let mut file_count: usize = 0;

                // Seed the stack with all selected folder paths.
                let mut stack: Vec<CatalogPath> = items
                    .iter()
                    .filter(|i| i.kind == ItemKind::Folder)
                    .map(|i| i.path.clone())
                    .collect();

                // Also count files that are directly selected.
                file_count += items.iter().filter(|i| i.kind == ItemKind::File).count();

                // Iterative DFS — avoids recursion and the `Send` bound.
                while let Some(path) = stack.pop() {
                    match api::list_folder(path).await {
                        Ok(data) => {
                            file_count += data.files.len();
                            for f in data.folders {
                                stack.push(f.path);
                            }
                        }
                        Err(e) => {
                            error_msg.set(Some(e.to_string()));
                            return;
                        }
                    }
                }

                modal.set(Some(ModalState::DeleteConfirm { items, file_count }));
            });
        }
    };

    let on_rename = move || {
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

    let on_move = move || {
        let items = selected.get_untracked();
        if !items.is_empty() {
            modal.set(Some(ModalState::Move { items }));
        }
    };

    view! {
        <div class="flex items-center gap-1.5 flex-wrap flex-shrink-0">
            // Always-visible buttons
            <ToolbarBtn
                icon="create_new_folder"
                label="New Folder"
                on_click=move || modal.set(Some(ModalState::CreateFolder))
            />
            <ToolbarBtn
                icon="upload"
                label="Upload"
                on_click=move || modal.set(Some(ModalState::Upload {
                    folder_path: current_path.get_untracked(),
                }))
            />

            // Separator — only visible when contextual buttons follow
            <Show when=has_selection>
                <div class="w-px h-5 bg-gray-200 mx-0.5" />
            </Show>

            // Download — only when exactly one file is selected
            <Show when=single_file_selected>
                <ToolbarBtn
                    icon="download"
                    label="Download"
                    on_click=on_download
                />
            </Show>

            // Rename — only when exactly one item is selected
            <Show when=single_selection>
                <ToolbarBtn
                    icon="drive_file_rename_outline"
                    label="Rename"
                    on_click=on_rename
                />
            </Show>

            // Move + Delete — any selection
            <Show when=has_selection>
                <ToolbarBtn
                    icon="drive_file_move"
                    label="Move"
                    on_click=on_move
                />
                <ToolbarBtn
                    icon="delete"
                    label="Delete"
                    danger=true
                    on_click=on_delete
                />
            </Show>

            // Selection count badge
            <Show when=has_selection>
                <span class="ml-1 text-xs text-gray-400 tabular-nums">
                    {move || format!("{} selected", selected.get().len())}
                </span>
            </Show>
        </div>
    }
}
