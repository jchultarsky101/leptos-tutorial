use common::CatalogPath;
use leptos::prelude::*;

use crate::api;
use crate::app::SelectedItem;

// ── Tree node ─────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct TreeNode {
    path: CatalogPath,
    name: String,
    depth: usize,
    expanded: bool,
    loading: bool,
}

// ── Path helpers ──────────────────────────────────────────────────────────────

/// Returns, in top-down order, every path that must be **expanded** to make
/// `path` visible as a tree node.
///
/// `/a/b/c` → `["/", "/a", "/a/b"]`  (`/a/b/c` itself is the target, not expanded)
fn ancestors_to_expand(path: &CatalogPath) -> Vec<CatalogPath> {
    let mut result = Vec::new();
    if path.is_root() {
        return result; // root is always visible; nothing to expand
    }
    result.push(CatalogPath::new("/").expect("root always valid"));

    let parts: Vec<&str> = path.as_str().trim_start_matches('/').split('/').collect();
    let mut cum = String::new();
    // Walk all segments except the last (which is the target itself).
    for part in parts.iter().take(parts.len().saturating_sub(1)) {
        cum.push('/');
        cum.push_str(part);
        if let Ok(p) = CatalogPath::new(&cum) {
            result.push(p);
        }
    }
    result
}

// ── Expand / collapse ─────────────────────────────────────────────────────────

/// Expand `path` in the flat node list, loading its children from the API.
/// Idempotent: bails early if already expanded or currently loading.
async fn do_expand(path: CatalogPath, nodes: RwSignal<Vec<TreeNode>>) {
    let should_proceed = nodes.with_untracked(|ns| {
        ns.iter()
            .find(|n| n.path == path)
            .map(|n| !n.expanded && !n.loading)
            .unwrap_or(false)
    });
    if !should_proceed {
        return;
    }

    // Mark loading.
    nodes.update(|ns| {
        if let Some(n) = ns.iter_mut().find(|n| n.path == path) {
            n.loading = true;
        }
    });

    let result = api::list_folder(path.clone()).await;

    nodes.update(|ns| {
        let Some(idx) = ns.iter().position(|n| n.path == path) else {
            return;
        };
        let child_depth = ns[idx].depth + 1;
        ns[idx].loading = false;
        ns[idx].expanded = true;

        if let Ok(data) = result {
            let new_children: Vec<TreeNode> = data
                .folders
                .into_iter()
                .map(|f| TreeNode {
                    name: f.path.name().to_owned(),
                    path: f.path,
                    depth: child_depth,
                    expanded: false,
                    loading: false,
                })
                .collect();

            // Since the node was collapsed, there are no descendants between
            // idx and the next sibling — inserting at idx+1 is always correct.
            let tail = ns.split_off(idx + 1);
            ns.extend(new_children);
            ns.extend(tail);
        }
    });
}

/// Collapse `path`: hide all its descendants and mark the node as collapsed.
fn do_collapse(path: CatalogPath, nodes: RwSignal<Vec<TreeNode>>) {
    nodes.update(|ns| {
        if let Some(n) = ns.iter_mut().find(|n| n.path == path) {
            n.expanded = false;
        }
        // Remove all strict descendants.
        ns.retain(|n| !n.path.starts_with_folder(&path) || n.path == path);
    });
}

// ── Component ─────────────────────────────────────────────────────────────────

#[component]
pub fn FolderTree() -> impl IntoView {
    let current_path =
        use_context::<RwSignal<CatalogPath>>().expect("current_path context missing");
    let selected = use_context::<RwSignal<Vec<SelectedItem>>>().expect("selected context missing");
    let catalog_version = use_context::<RwSignal<u32>>().expect("catalog_version context missing");

    // Flat, ordered list of visible tree nodes.
    let nodes: RwSignal<Vec<TreeNode>> = RwSignal::new(vec![TreeNode {
        path: CatalogPath::new("/").expect("root always valid"),
        name: "Root".into(),
        depth: 0,
        expanded: false,
        loading: false,
    }]);

    // On mount and after every catalog mutation: reset the tree to just the
    // root node, then re-expand root + every ancestor of the current folder.
    // This keeps the tree in sync after creates, renames, moves, and deletes.
    Effect::new(move |_| {
        let _v = catalog_version.get(); // subscribe so the effect re-runs on change
        let target = current_path.get_untracked();
        nodes.set(vec![TreeNode {
            path: CatalogPath::new("/").expect("root always valid"),
            name: "Root".into(),
            depth: 0,
            expanded: false,
            loading: false,
        }]);
        wasm_bindgen_futures::spawn_local(async move {
            do_expand(CatalogPath::new("/").expect("root always valid"), nodes).await;
            for ancestor in ancestors_to_expand(&target) {
                do_expand(ancestor, nodes).await;
            }
        });
    });

    // Whenever current_path changes (pure navigation), expand every ancestor
    // so the active folder is visible and highlighted in the tree.
    Effect::new(move |_| {
        let target = current_path.get();
        wasm_bindgen_futures::spawn_local(async move {
            for ancestor in ancestors_to_expand(&target) {
                do_expand(ancestor, nodes).await;
            }
        });
    });

    view! {
        <div class="overflow-y-auto select-none py-1">
            {move || {
                let cp = current_path.get();
                nodes
                    .get()
                    .into_iter()
                    .map(move |node| {
                        let is_current = node.path == cp;
                        let is_expanded = node.expanded;
                        let is_loading = node.loading;
                        let path_for_chevron = node.path.clone();
                        let path_for_nav = node.path.clone();

                        let expand_icon = if is_loading {
                            "progress_activity"
                        } else if is_expanded {
                            "expand_more"
                        } else {
                            "chevron_right"
                        };
                        let folder_icon = if is_current { "folder_open" } else { "folder" };
                        let indent_px = node.depth * 16;

                        view! {
                            <div
                                class=if is_current {
                                    "flex items-center gap-0.5 pr-2 py-0.5 rounded mx-1 \
                                     bg-gray-900 text-white cursor-pointer"
                                } else {
                                    "flex items-center gap-0.5 pr-2 py-0.5 rounded mx-1 \
                                     text-gray-600 dark:text-gray-300 \
                                     hover:bg-gray-100 dark:hover:bg-gray-700 cursor-pointer"
                                }
                                style=format!("padding-left: {}px;", 4 + indent_px)
                            >
                                // ── Expand / collapse toggle ───────────────
                                <button
                                    class=if is_current {
                                        "w-5 h-5 flex-shrink-0 flex items-center justify-center \
                                         opacity-70 hover:opacity-100"
                                    } else {
                                        "w-5 h-5 flex-shrink-0 flex items-center justify-center \
                                         text-gray-400 dark:text-gray-500 \
                                         hover:text-gray-700 dark:hover:text-gray-300"
                                    }
                                    on:click=move |ev| {
                                        ev.stop_propagation();
                                        if is_expanded {
                                            do_collapse(path_for_chevron.clone(), nodes);
                                        } else {
                                            let p = path_for_chevron.clone();
                                            wasm_bindgen_futures::spawn_local(async move {
                                                do_expand(p, nodes).await;
                                            });
                                        }
                                    }
                                >
                                    <span class="material-symbols-outlined" style="font-size:14px;">
                                        {expand_icon}
                                    </span>
                                </button>

                                // ── Folder icon + name (navigate on click) ─
                                <button
                                    class="flex items-center gap-1 flex-1 min-w-0 text-left"
                                    on:click=move |_| {
                                        selected.set(Vec::new());
                                        current_path.set(path_for_nav.clone());
                                        // Also expand the clicked folder to reveal children.
                                        if !is_expanded {
                                            let p = path_for_nav.clone();
                                            wasm_bindgen_futures::spawn_local(async move {
                                                do_expand(p, nodes).await;
                                            });
                                        }
                                    }
                                >
                                    <span
                                        class="material-symbols-outlined"
                                        style="font-size:16px;"
                                    >
                                        {folder_icon}
                                    </span>
                                    <span class="text-xs font-medium truncate">
                                        {node.name.clone()}
                                    </span>
                                </button>
                            </div>
                        }
                    })
                    .collect_view()
            }}
        </div>
    }
}
