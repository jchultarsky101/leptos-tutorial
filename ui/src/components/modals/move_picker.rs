use common::{CatalogPath, dto::PatchFileRequest, dto::PatchFolderRequest};
use leptos::prelude::*;

use crate::api;
use crate::app::{ContentsResource, ItemKind, ModalState, SelectedItem};

#[derive(Clone, Debug)]
pub struct PickerNode {
    pub path: CatalogPath,
    pub name: String,
    pub indent: usize,
    pub expanded: bool,
    pub loading: bool,
}

/// Expand or collapse a node at `idx` in the flat `nodes` list.
/// `items_to_skip` are the paths being moved (excluded from destination tree).
fn toggle_expand(idx: usize, nodes: RwSignal<Vec<PickerNode>>, items_to_skip: Vec<SelectedItem>) {
    let current = nodes.get_untracked();
    let Some(node) = current.get(idx) else { return };

    if node.expanded {
        // Collapse: remove all descendants of this node.
        let parent_path = node.path.clone();
        nodes.update(|ns| {
            if let Some(n) = ns.get_mut(idx) {
                n.expanded = false;
            }
            ns.retain(|n| !n.path.starts_with_folder(&parent_path) || n.path == parent_path);
        });
        return;
    }

    // Expand: fetch children asynchronously and insert them.
    let path = node.path.clone();
    let indent = node.indent + 1;

    nodes.update(|ns| {
        if let Some(n) = ns.get_mut(idx) {
            n.loading = true;
        }
    });

    wasm_bindgen_futures::spawn_local(async move {
        let result = api::list_folder(path.clone()).await;
        nodes.update(|ns| {
            if let Some(n) = ns.iter_mut().find(|n| n.path == path) {
                n.expanded = true;
                n.loading = false;
            }
            let insert_after = ns.iter().position(|n| n.path == path).map(|i| i + 1);
            if let (Ok(data), Some(pos)) = (result, insert_after) {
                let new_nodes: Vec<PickerNode> = data
                    .folders
                    .into_iter()
                    .filter(|f| {
                        !items_to_skip
                            .iter()
                            .any(|it| f.path == it.path || f.path.starts_with_folder(&it.path))
                    })
                    .map(|f| PickerNode {
                        name: f.path.name().to_owned(),
                        path: f.path,
                        indent,
                        expanded: false,
                        loading: false,
                    })
                    .collect();
                let tail = ns.split_off(pos);
                ns.extend(new_nodes);
                ns.extend(tail);
            }
        });
    });
}

#[component]
pub fn MovePickerModal(items: Vec<SelectedItem>) -> impl IntoView {
    let modal = use_context::<RwSignal<Option<ModalState>>>().expect("modal context missing");
    let selected = use_context::<RwSignal<Vec<SelectedItem>>>().expect("selected context missing");
    let contents = use_context::<ContentsResource>().expect("contents context missing");
    let error_msg = use_context::<RwSignal<Option<String>>>().expect("error_msg context missing");
    let catalog_version = use_context::<RwSignal<u32>>().expect("catalog_version context missing");

    // Flat list of visible picker nodes.
    let nodes: RwSignal<Vec<PickerNode>> = RwSignal::new(Vec::new());
    let selected_dest: RwSignal<Option<CatalogPath>> = RwSignal::new(None);
    let submitting = RwSignal::new(false);

    // Load root children on mount.
    {
        let items_for_init = items.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let root = CatalogPath::new("/").expect("root always valid");
            if let Ok(data) = api::list_folder(root.clone()).await {
                let root_node = PickerNode {
                    path: root,
                    name: "Root".into(),
                    indent: 0,
                    expanded: true,
                    loading: false,
                };
                let mut initial: Vec<PickerNode> = vec![root_node];
                for folder in data.folders {
                    let is_source = items_for_init.iter().any(|it| {
                        folder.path == it.path || folder.path.starts_with_folder(&it.path)
                    });
                    if !is_source {
                        initial.push(PickerNode {
                            name: folder.path.name().to_owned(),
                            path: folder.path,
                            indent: 1,
                            expanded: false,
                            loading: false,
                        });
                    }
                }
                nodes.set(initial);
            }
        });
    }

    let items_for_move = items.clone();
    let on_move = move |_| {
        let Some(dest) = selected_dest.get_untracked() else {
            return;
        };
        let mv_items = items_for_move.clone();
        submitting.set(true);

        wasm_bindgen_futures::spawn_local(async move {
            let mut had_error = false;
            for item in mv_items {
                let result = match item.kind {
                    ItemKind::Folder => api::patch_folder(
                        item.path,
                        PatchFolderRequest {
                            name: None,
                            new_parent_path: Some(dest.clone()),
                        },
                    )
                    .await
                    .map(|_| ()),
                    ItemKind::File => api::patch_file(
                        item.path,
                        PatchFileRequest {
                            name: None,
                            new_folder_path: Some(dest.clone()),
                        },
                    )
                    .await
                    .map(|_| ()),
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
                class="bg-white rounded-lg shadow-xl p-6 w-full max-w-md mx-4 \
                       max-h-[80vh] flex flex-col"
                on:click=|ev| ev.stop_propagation()
            >
                // Header
                <div class="flex items-center gap-2 mb-4">
                    <span class="material-symbols-outlined text-gray-500">"drive_file_move"</span>
                    <h2 class="text-sm font-semibold text-gray-900">"Move to…"</h2>
                </div>

                <div class="flex-1 overflow-y-auto border border-gray-200 rounded mb-4">
                    {move || {
                        let items_snap = items.clone();
                        nodes.get().into_iter().enumerate().map(move |(idx, node)| {
                            let path_for_select = node.path.clone();
                            let is_selected = move || {
                                selected_dest.get().as_ref() == Some(&path_for_select)
                            };
                            let indent_px = node.indent * 20;
                            let items_for_expand = items_snap.clone();

                            // Expand/collapse icon name
                            let expand_icon = if node.loading {
                                "progress_activity"
                            } else if node.expanded {
                                "expand_more"
                            } else {
                                "chevron_right"
                            };

                            view! {
                                <div
                                    class=move || {
                                        if is_selected() {
                                            "flex items-center gap-1.5 px-2 py-1.5 cursor-pointer \
                                             bg-gray-100 border-l-2 border-gray-900".to_owned()
                                        } else {
                                            "flex items-center gap-1.5 px-2 py-1.5 cursor-pointer \
                                             hover:bg-gray-50".to_owned()
                                        }
                                    }
                                    style=format!("padding-left: {}px", 8 + indent_px)
                                    on:click={
                                        let p = node.path.clone();
                                        move |_| selected_dest.set(Some(p.clone()))
                                    }
                                >
                                    <button
                                        class="w-4 h-4 text-gray-400 hover:text-gray-700 \
                                               flex-shrink-0 flex items-center justify-center"
                                        on:click=move |ev| {
                                            ev.stop_propagation();
                                            toggle_expand(idx, nodes, items_for_expand.clone());
                                        }
                                    >
                                        <span class="material-symbols-outlined"
                                            style="font-size:16px;">
                                            {expand_icon}
                                        </span>
                                    </button>
                                    <span class="material-symbols-outlined text-gray-400"
                                        style="font-size:16px;">
                                        "folder"
                                    </span>
                                    <span class="text-sm text-gray-800">{node.name.clone()}</span>
                                </div>
                            }
                        }).collect_view()
                    }}
                </div>

                <div class="flex gap-2 justify-end">
                    <button
                        class="px-3 py-1.5 text-sm text-gray-500 hover:text-gray-900 \
                               transition-colors"
                        on:click=move |_| modal.set(None)
                    >
                        "Cancel"
                    </button>
                    <button
                        class="px-3 py-1.5 text-sm font-medium bg-gray-900 text-white \
                               rounded hover:bg-gray-700 disabled:opacity-40 transition-colors"
                        prop:disabled=move || submitting.get() || selected_dest.get().is_none()
                        on:click=on_move
                    >
                        {move || if submitting.get() { "Moving…" } else { "Move Here" }}
                    </button>
                </div>
            </div>
        </div>
    }
}
