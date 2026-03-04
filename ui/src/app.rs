use common::{CatalogPath, dto::FolderContentsDto};
use leptos::prelude::*;
use wasm_bindgen::prelude::*;

use crate::{
    api,
    components::{Breadcrumb, FileGrid, FilePreview, FolderTree, Modals, Toolbar},
    error::UiError,
};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ItemKind {
    Folder,
    File,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct SelectedItem {
    pub path: CatalogPath,
    pub kind: ItemKind,
}

#[derive(Clone, Debug)]
pub enum ModalState {
    CreateFolder,
    Rename {
        path: CatalogPath,
        current_name: String,
        kind: ItemKind,
    },
    Move {
        items: Vec<SelectedItem>,
    },
    Upload {
        folder_path: CatalogPath,
    },
    DeleteConfirm {
        items: Vec<SelectedItem>,
        /// Number of files that will be removed inside selected folders.
        file_count: usize,
    },
}

/// Identifies the file currently open in the preview pane.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewTarget {
    pub path: CatalogPath,
    pub content_type: String,
}

/// A CSR-local resource (no `Send` requirement on the future).
pub type ContentsResource = LocalResource<Result<FolderContentsDto, UiError>>;

const MIN_TREE_W: f64 = 140.0;
const DEFAULT_TREE_W: f64 = 220.0;
const MIN_PREVIEW_W: f64 = 280.0;
const DEFAULT_PREVIEW_W: f64 = 400.0;

#[component]
pub fn App() -> impl IntoView {
    let current_path = RwSignal::new(CatalogPath::new("/").expect("root path is always valid"));
    let selected: RwSignal<Vec<SelectedItem>> = RwSignal::new(Vec::new());
    let modal: RwSignal<Option<ModalState>> = RwSignal::new(None);
    let error_msg: RwSignal<Option<String>> = RwSignal::new(None);

    // LocalResource tracks `current_path` reactively inside the closure.
    let contents: ContentsResource = LocalResource::new(move || {
        let path = current_path.get();
        async move { api::list_folder(path).await }
    });

    // Incremented after every catalog mutation so FolderTree can refresh.
    let catalog_version: RwSignal<u32> = RwSignal::new(0_u32);

    // File currently open in the preview pane (None = pane closed).
    let preview_file: RwSignal<Option<PreviewTarget>> = RwSignal::new(None);

    provide_context(current_path);
    provide_context(selected);
    provide_context(modal);
    provide_context(error_msg);
    provide_context(contents);
    provide_context(catalog_version);
    provide_context(preview_file);

    // Clear the preview whenever the user navigates to a different folder.
    Effect::new(move |_| {
        let _ = current_path.get();
        preview_file.set(None);
    });

    // ── Tree panel resize state ────────────────────────────────────────────────
    let tree_w: RwSignal<f64> = RwSignal::new(DEFAULT_TREE_W);
    let drag_active: RwSignal<bool> = RwSignal::new(false);
    let drag_x0: RwSignal<f64> = RwSignal::new(0.0);
    let drag_w0: RwSignal<f64> = RwSignal::new(0.0);

    // ── Preview panel resize state ────────────────────────────────────────────
    let preview_w: RwSignal<f64> = RwSignal::new(DEFAULT_PREVIEW_W);
    let preview_drag_active: RwSignal<bool> = RwSignal::new(false);
    let preview_drag_x0: RwSignal<f64> = RwSignal::new(0.0);
    let preview_drag_w0: RwSignal<f64> = RwSignal::new(0.0);

    // Window-level listeners — both tree and preview drag share the same
    // mousemove/mouseup events; each closure checks its own active flag.
    {
        let on_move = Closure::<dyn Fn(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
            let x = e.client_x() as f64;
            if drag_active.get_untracked() {
                let delta = x - drag_x0.get_untracked();
                tree_w.set((drag_w0.get_untracked() + delta).max(MIN_TREE_W));
            }
            if preview_drag_active.get_untracked() {
                // Divider is to the LEFT of the preview panel; moving left
                // (negative delta) widens the panel.
                let delta = x - preview_drag_x0.get_untracked();
                preview_w.set((preview_drag_w0.get_untracked() - delta).max(MIN_PREVIEW_W));
            }
        });
        let on_up = Closure::<dyn Fn(web_sys::MouseEvent)>::new(move |_: web_sys::MouseEvent| {
            drag_active.set(false);
            preview_drag_active.set(false);
        });
        let win = web_sys::window().expect("no window");
        let _ = win.add_event_listener_with_callback("mousemove", on_move.as_ref().unchecked_ref());
        let _ = win.add_event_listener_with_callback("mouseup", on_up.as_ref().unchecked_ref());
        on_move.forget();
        on_up.forget();
    }

    view! {
        // Full-viewport flex column — no scroll on the outer shell.
        <div class="h-screen flex flex-col bg-gray-50 overflow-hidden">
            // Drag overlay — shown whenever any panel resize is in progress.
            <Show when=move || drag_active.get() || preview_drag_active.get()>
                <div class="fixed inset-0 z-50 cursor-col-resize" />
            </Show>

            // ── App header ────────────────────────────────────────────────────
            <header class="flex-shrink-0 bg-gray-900 \
                           px-6 py-3 flex items-center gap-3">
                <img src="/assets/logo.svg" alt="" class="h-6 w-6" />
                <h1 class="text-base font-semibold tracking-tight text-white">
                    "File Catalog"
                </h1>
            </header>

            // ── Error banner (conditionally shown) ───────────────────────────
            <Show when=move || error_msg.get().is_some()>
                <div class="flex-shrink-0 px-6 py-2 bg-red-50 border-b border-red-200 \
                             flex items-center justify-between gap-3">
                    <div class="flex items-center gap-2">
                        <span class="material-symbols-outlined text-red-500"
                            style="font-size:18px;">"error"</span>
                        <span class="text-sm text-red-700">
                            {move || error_msg.get().unwrap_or_default()}
                        </span>
                    </div>
                    <button
                        class="text-red-400 hover:text-red-600 focus:outline-none"
                        on:click=move |_| error_msg.set(None)
                    >
                        <span class="material-symbols-outlined" style="font-size:18px;">
                            "close"
                        </span>
                    </button>
                </div>
            </Show>

            // ── Breadcrumb (left) + Toolbar (right) in one row ───────────────
            <div class="flex-shrink-0 bg-white border-b border-gray-100 \
                         px-4 flex items-center justify-between gap-4 min-h-[48px]">
                <div class="flex-1 min-w-0">
                    <Breadcrumb />
                </div>
                <Toolbar />
            </div>

            // ── Main content area: tree | divider | file grid ─────────────────
            <div class="flex-1 min-h-0 flex gap-0 p-3">
                // Folder tree panel
                <div
                    class="flex-shrink-0 bg-white border border-gray-200 rounded-lg \
                           shadow-sm flex flex-col overflow-hidden"
                    style=move || format!("width: {}px;", tree_w.get())
                >
                    <div class="px-3 py-1.5 border-b border-gray-100 flex-shrink-0">
                        <span class="text-xs font-medium text-gray-400 uppercase tracking-wider">
                            "Folders"
                        </span>
                    </div>
                    <div class="flex-1 overflow-y-auto">
                        <FolderTree />
                    </div>
                </div>

                // Drag divider
                <div
                    class="w-2 flex-shrink-0 self-stretch cursor-col-resize \
                           hover:bg-gray-200 transition-colors mx-1 rounded"
                    on:mousedown=move |e: web_sys::MouseEvent| {
                        e.prevent_default();
                        drag_active.set(true);
                        drag_x0.set(e.client_x() as f64);
                        drag_w0.set(tree_w.get_untracked());
                    }
                />

                // File grid (grows to fill remaining space, manages its own height)
                <div class="flex-1 min-w-0 min-h-0 flex flex-col">
                    <FileGrid />
                </div>

                // ── Preview panel (shown when a file is selected for preview) ─
                <Show when=move || preview_file.get().is_some()>
                    // Drag divider — left edge of the preview panel.
                    <div
                        class="w-2 flex-shrink-0 self-stretch cursor-col-resize \
                               hover:bg-gray-200 transition-colors mx-1 rounded"
                        on:mousedown=move |e: web_sys::MouseEvent| {
                            e.prevent_default();
                            preview_drag_active.set(true);
                            preview_drag_x0.set(e.client_x() as f64);
                            preview_drag_w0.set(preview_w.get_untracked());
                        }
                    />
                    <div
                        class="flex-shrink-0 bg-white border border-gray-200 \
                               rounded-lg shadow-sm flex flex-col overflow-hidden"
                        style=move || format!("width: {}px;", preview_w.get())
                    >
                        <FilePreview />
                    </div>
                </Show>
            </div>

            <Modals />
        </div>
    }
}
