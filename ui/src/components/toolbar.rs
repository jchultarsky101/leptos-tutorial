use common::CatalogPath;
use leptos::prelude::*;

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

    let has_selection = move || !selected.get().is_empty();
    let single_selection = move || selected.get().len() == 1;

    // Delete all selected items.
    let on_delete = move || {
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
        <div class="flex items-center gap-1.5 mb-4 flex-wrap">
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

            // Separator — only visible when contextual buttons are shown
            <Show when=has_selection>
                <div class="w-px h-5 bg-gray-200 mx-0.5" />
            </Show>

            // Selection-conditional buttons
            <Show when=single_selection>
                <ToolbarBtn
                    icon="drive_file_rename_outline"
                    label="Rename"
                    on_click=on_rename
                />
            </Show>
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
