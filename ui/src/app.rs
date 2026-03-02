use common::{CatalogPath, dto::FolderContentsDto};
use leptos::prelude::*;

use crate::{
    api,
    components::{Breadcrumb, FileGrid, Modals, Toolbar},
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
}

/// A CSR-local resource (no `Send` requirement on the future).
pub type ContentsResource = LocalResource<Result<FolderContentsDto, UiError>>;

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

    provide_context(current_path);
    provide_context(selected);
    provide_context(modal);
    provide_context(error_msg);
    provide_context(contents);

    view! {
        <div class="min-h-screen bg-gray-50">
            <header class="bg-white border-b border-gray-200 px-6 py-4 flex items-center gap-3">
                <span class="text-2xl">"🗂"</span>
                <h1 class="text-xl font-semibold text-gray-800">"File Catalog"</h1>
            </header>
            <main class="px-6 py-4 max-w-7xl mx-auto">
                <Show when=move || error_msg.get().is_some()>
                    <div class="mb-4 p-3 bg-red-50 border border-red-200 rounded-md flex items-center justify-between">
                        <span class="text-sm text-red-700">
                            {move || error_msg.get().unwrap_or_default()}
                        </span>
                        <button
                            class="text-red-400 hover:text-red-600 ml-4 font-bold"
                            on:click=move |_| error_msg.set(None)
                        >
                            "✕"
                        </button>
                    </div>
                </Show>
                <Breadcrumb />
                <Toolbar />
                <FileGrid />
            </main>
            <Modals />
        </div>
    }
}
