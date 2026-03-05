use common::{
    CatalogPath,
    dto::{FolderContentsDto, SearchResultsDto},
};
use leptos::prelude::*;
use wasm_bindgen::prelude::*;

use crate::{
    api,
    components::{
        Breadcrumb, FileGrid, FilePreview, FolderTree, HamburgerMenu, Modals, SearchBar,
        SearchResults, StatsModal, Toolbar,
    },
    error::UiError,
};

// ── Theme ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Theme {
    Light,
    Dark,
}

// ── App-wide settings ─────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SortField {
    Name,
    Date,
    Size,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GlobalSortDir {
    Asc,
    Desc,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DateFormat {
    Relative,
    Absolute,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppSettings {
    pub sort_field: SortField,
    pub sort_dir: GlobalSortDir,
    pub date_format: DateFormat,
    pub preview_auto_open: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            sort_field: SortField::Name,
            sort_dir: GlobalSortDir::Asc,
            date_format: DateFormat::Relative,
            preview_auto_open: false,
        }
    }
}

// ── localStorage helpers ──────────────────────────────────────────────────────

fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten()
}

pub fn storage_get(key: &str) -> Option<String> {
    local_storage().and_then(|s| s.get_item(key).ok()).flatten()
}

pub fn storage_set(key: &str, value: &str) {
    if let Some(s) = local_storage() {
        let _ = s.set_item(key, value);
    }
}

fn load_theme() -> Theme {
    match storage_get("theme").as_deref() {
        Some("dark") => Theme::Dark,
        _ => Theme::Light,
    }
}

fn load_settings() -> AppSettings {
    let sort_field = match storage_get("sort_field").as_deref() {
        Some("date") => SortField::Date,
        Some("size") => SortField::Size,
        _ => SortField::Name,
    };
    let sort_dir = match storage_get("sort_dir").as_deref() {
        Some("desc") => GlobalSortDir::Desc,
        _ => GlobalSortDir::Asc,
    };
    let date_format = match storage_get("date_format").as_deref() {
        Some("absolute") => DateFormat::Absolute,
        _ => DateFormat::Relative,
    };
    let preview_auto_open = storage_get("preview_auto_open").as_deref() != Some("false");
    AppSettings {
        sort_field,
        sort_dir,
        date_format,
        preview_auto_open,
    }
}

/// Apply or remove the `.dark` class on `<html>`.
fn apply_theme(theme: &Theme) {
    let Some(win) = web_sys::window() else {
        return;
    };
    let Some(doc) = win.document() else { return };
    let Some(root) = doc.document_element() else {
        return;
    };
    match theme {
        Theme::Dark => {
            let _ = root.class_list().add_1("dark");
        }
        Theme::Light => {
            let _ = root.class_list().remove_1("dark");
        }
    }
}

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
    // ── Theme & settings (localStorage-backed) ────────────────────────────────
    let theme: RwSignal<Theme> = RwSignal::new(load_theme());
    let settings: RwSignal<AppSettings> = RwSignal::new(load_settings());
    let stats_open: RwSignal<bool> = RwSignal::new(false);

    // Apply the initial theme immediately (before first render).
    apply_theme(&theme.get_untracked());

    provide_context(theme);
    provide_context(settings);
    provide_context(stats_open);

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

    // ── Search state ──────────────────────────────────────────────────────────
    let search_query: RwSignal<Option<String>> = RwSignal::new(None);
    let search_fuzzy: RwSignal<bool> = RwSignal::new(false);
    let search_results: RwSignal<Option<SearchResultsDto>> = RwSignal::new(None);

    provide_context(current_path);
    provide_context(selected);
    provide_context(modal);
    provide_context(error_msg);
    provide_context(contents);
    provide_context(catalog_version);
    provide_context(preview_file);
    provide_context(search_query);
    provide_context(search_fuzzy);
    provide_context(search_results);

    // Clear the preview whenever the user navigates to a different folder.
    Effect::new(move |_| {
        let _ = current_path.get();
        preview_file.set(None);
    });

    // Keep the `dark` class on <html> in sync with the theme signal.
    Effect::new(move |_| {
        apply_theme(&theme.get());
    });

    // ── Search resource ────────────────────────────────────────────────────────
    let _search_resource: LocalResource<()> = LocalResource::new(move || {
        let query = search_query.get();
        let fuzzy = search_fuzzy.get();
        async move {
            match query {
                Some(q) if !q.trim().is_empty() => match api::search(q, fuzzy).await {
                    Ok(dto) => search_results.set(Some(dto)),
                    Err(e) => {
                        tracing::warn!("search error: {e}");
                        search_results.set(None);
                    }
                },
                _ => {
                    search_results.set(None);
                }
            }
        }
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

    // Whether search is active (query is set and non-empty).
    let search_active = move || search_query.get().is_some();

    view! {
        // Full-viewport flex column — no scroll on the outer shell.
        <div class="h-screen flex flex-col bg-gray-50 dark:bg-gray-900 overflow-hidden">
            // Drag overlay — shown whenever any panel resize is in progress.
            <Show when=move || drag_active.get() || preview_drag_active.get()>
                <div class="fixed inset-0 z-50 cursor-col-resize" />
            </Show>

            // ── App header ────────────────────────────────────────────────────
            <header class="flex-shrink-0 bg-gray-900 \
                           px-4 py-3 flex items-center gap-3">
                <img src="/assets/logo.svg" alt="" class="h-6 w-6" />
                <h1 class="text-base font-semibold tracking-tight text-white">
                    "File Catalog"
                </h1>
                <HamburgerMenu />
            </header>

            // ── Search bar ────────────────────────────────────────────────────
            <SearchBar />

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
            <Show when=move || !search_active()>
                <div class="flex-shrink-0 bg-white dark:bg-gray-800 border-b \
                             border-gray-100 dark:border-gray-700 \
                             px-4 flex items-center justify-between gap-4 min-h-[48px]">
                    <div class="flex-1 min-w-0">
                        <Breadcrumb />
                    </div>
                    <Toolbar />
                </div>
            </Show>

            // ── Main content area ─────────────────────────────────────────────
            <Show
                when=search_active
                fallback=move || view! {
                    <div class="flex-1 min-h-0 flex gap-0 p-3">
                        // Folder tree panel
                        <div
                            class="flex-shrink-0 bg-white dark:bg-gray-800 border \
                                   border-gray-200 dark:border-gray-700 rounded-lg \
                                   shadow-sm flex flex-col overflow-hidden"
                            style=move || format!("width: {}px;", tree_w.get())
                        >
                            <div class="px-3 py-1.5 border-b border-gray-100 \
                                        dark:border-gray-700 flex-shrink-0">
                                <span class="text-xs font-medium text-gray-400 \
                                             dark:text-gray-500 uppercase tracking-wider">
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

                        // File grid (grows to fill remaining space)
                        <div class="flex-1 min-w-0 min-h-0 flex flex-col">
                            <FileGrid />
                        </div>

                        // ── Preview panel ─────────────────────────────────────
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
                                class="flex-shrink-0 bg-white dark:bg-gray-800 border \
                                       border-gray-200 dark:border-gray-700 \
                                       rounded-lg shadow-sm flex flex-col overflow-hidden"
                                style=move || format!("width: {}px;", preview_w.get())
                            >
                                <FilePreview />
                            </div>
                        </Show>
                    </div>
                }
            >
                // ── Search results view ───────────────────────────────────────
                <div class="flex-1 min-h-0 p-3">
                    <SearchResults />
                </div>
            </Show>

            <Modals />
            <StatsModal />
        </div>
    }
}
