use common::CatalogPath;
use leptos::prelude::*;
use wasm_bindgen::prelude::*;

use crate::app::{ContentsResource, ItemKind, SelectedItem};

// ── Sort state ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortCol {
    Name,
    Size,
    Modified,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortDir {
    Asc,
    Desc,
}

impl SortDir {
    fn flip(self) -> Self {
        match self {
            Self::Asc => Self::Desc,
            Self::Desc => Self::Asc,
        }
    }
    fn arrow(self) -> &'static str {
        match self {
            Self::Asc => " ▲",
            Self::Desc => " ▼",
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Returns a Material Symbol ligature name for the given MIME type.
fn file_icon(content_type: &str) -> &'static str {
    if content_type.starts_with("image/") {
        "image"
    } else if content_type.starts_with("video/") {
        "movie"
    } else if content_type.starts_with("audio/") {
        "audio_file"
    } else if content_type.contains("pdf") {
        "picture_as_pdf"
    } else if content_type.contains("zip")
        || content_type.contains("tar")
        || content_type.contains("gz")
    {
        "folder_zip"
    } else {
        "description"
    }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn short_date(rfc3339: &str) -> String {
    rfc3339.get(..10).unwrap_or(rfc3339).to_owned()
}

// ── Column resize ─────────────────────────────────────────────────────────────

const MIN_COL_W: f64 = 60.0;

// ── Component ─────────────────────────────────────────────────────────────────

#[component]
pub fn FileGrid() -> impl IntoView {
    let current_path =
        use_context::<RwSignal<CatalogPath>>().expect("current_path context missing");
    let selected = use_context::<RwSignal<Vec<SelectedItem>>>().expect("selected context missing");
    let contents = use_context::<ContentsResource>().expect("contents context missing");

    // ── Sort ──────────────────────────────────────────────────────────────────

    let sort_col: RwSignal<SortCol> = RwSignal::new(SortCol::Name);
    let sort_dir: RwSignal<SortDir> = RwSignal::new(SortDir::Asc);

    let toggle_sort = move |col: SortCol| {
        if sort_col.get_untracked() == col {
            sort_dir.update(|d| *d = d.flip());
        } else {
            sort_col.set(col);
            sort_dir.set(SortDir::Asc);
        }
    };

    // ── Column widths (px) ────────────────────────────────────────────────────

    let name_w: RwSignal<f64> = RwSignal::new(320.0);
    let size_w: RwSignal<f64> = RwSignal::new(80.0);
    let modified_w: RwSignal<f64> = RwSignal::new(140.0);

    // ── Drag state ────────────────────────────────────────────────────────────

    let drag_col: RwSignal<Option<RwSignal<f64>>> = RwSignal::new(None);
    let drag_x0: RwSignal<f64> = RwSignal::new(0.0);
    let drag_w0: RwSignal<f64> = RwSignal::new(0.0);

    {
        let on_move = Closure::<dyn Fn(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
            if let Some(col) = drag_col.get_untracked() {
                let delta = e.client_x() as f64 - drag_x0.get_untracked();
                col.set((drag_w0.get_untracked() + delta).max(MIN_COL_W));
            }
        });
        let on_up = Closure::<dyn Fn(web_sys::MouseEvent)>::new(move |_: web_sys::MouseEvent| {
            drag_col.set(None);
        });
        let win = web_sys::window().expect("no window");
        let _ = win.add_event_listener_with_callback("mousemove", on_move.as_ref().unchecked_ref());
        let _ = win.add_event_listener_with_callback("mouseup", on_up.as_ref().unchecked_ref());
        on_move.forget();
        on_up.forget();
    }

    let start_resize = move |col_w: RwSignal<f64>, e: web_sys::MouseEvent| {
        e.prevent_default();
        drag_col.set(Some(col_w));
        drag_x0.set(e.client_x() as f64);
        drag_w0.set(col_w.get_untracked());
    };

    // ── Selection ─────────────────────────────────────────────────────────────

    let toggle_select = move |item: SelectedItem| {
        selected.update(|sel| {
            if let Some(pos) = sel.iter().position(|s| *s == item) {
                sel.remove(pos);
            } else {
                sel.push(item);
            }
        });
    };

    // ── View ──────────────────────────────────────────────────────────────────
    //
    // The outer container is a flex column that fills its parent (which is a
    // flex-1 column in app.rs).  The scrollable area takes all remaining height
    // via `flex-1 min-h-0 overflow-auto`; the stats footer is pinned at the
    // bottom via `flex-shrink-0`.

    view! {
        // Column-resize drag overlay — fixed, outside normal flow.
        <Show when=move || drag_col.get().is_some()>
            <div class="fixed inset-0 z-50 cursor-col-resize" />
        </Show>

        <div class="flex flex-col min-h-0 h-full \
                    bg-white rounded-lg shadow-sm border border-gray-200 overflow-hidden">

            // ── Scrollable table area ─────────────────────────────────────────
            <div class="flex-1 min-h-0 overflow-auto">
                {move || {
                    let col = sort_col.get();
                    let dir = sort_dir.get();

                    let resolved = contents.map(|r| r.clone());
                    match resolved {
                        None => view! {
                            <div class="p-10 text-center text-gray-400">
                                <span class="material-symbols-outlined text-gray-300"
                                    style="font-size:40px; display:block; margin-bottom:8px;">
                                    "hourglass_empty"
                                </span>
                                "Loading…"
                            </div>
                        }
                        .into_any(),

                        Some(Err(ref e)) => {
                            let msg = e.to_string();
                            view! {
                                <div class="p-10 text-center text-red-600">
                                    <span class="material-symbols-outlined text-red-300"
                                        style="font-size:40px; display:block; margin-bottom:8px;">
                                        "error"
                                    </span>
                                    {msg}
                                </div>
                            }
                            .into_any()
                        }

                        Some(Ok(data)) => {
                            if data.folders.is_empty() && data.files.is_empty() {
                                return view! {
                                    <div class="p-10 text-center text-gray-400">
                                        <span class="material-symbols-outlined text-gray-300"
                                            style="font-size:40px; display:block; margin-bottom:8px;">
                                            "inbox"
                                        </span>
                                        "This folder is empty"
                                    </div>
                                }
                                .into_any();
                            }

                            let mut folders = data.folders.clone();
                            folders.sort_by(|a, b| {
                                let ord = match col {
                                    SortCol::Name | SortCol::Size => {
                                        a.path.name().cmp(b.path.name())
                                    }
                                    SortCol::Modified => a.modified_at.cmp(&b.modified_at),
                                };
                                if dir == SortDir::Desc { ord.reverse() } else { ord }
                            });

                            let mut files = data.files.clone();
                            files.sort_by(|a, b| {
                                let ord = match col {
                                    SortCol::Name => a.path.name().cmp(b.path.name()),
                                    SortCol::Size => a.size_bytes.cmp(&b.size_bytes),
                                    SortCol::Modified => a.modified_at.cmp(&b.modified_at),
                                };
                                if dir == SortDir::Desc { ord.reverse() } else { ord }
                            });

                            let label = |c: SortCol, text: &'static str| -> String {
                                if c == col {
                                    format!("{text}{}", dir.arrow())
                                } else {
                                    text.to_owned()
                                }
                            };
                            let name_label = label(SortCol::Name, "Name");
                            let size_label = label(SortCol::Size, "Size");
                            let modified_label = label(SortCol::Modified, "Modified");

                            let hdr_cls = |c: SortCol, extra: &'static str| -> String {
                                let color = if c == col {
                                    "text-gray-900 font-semibold"
                                } else {
                                    "text-gray-400 font-medium"
                                };
                                format!(
                                    "relative px-3 py-2.5 text-xs uppercase tracking-wider \
                                     cursor-pointer select-none bg-gray-50 \
                                     border-r border-gray-200 {color} {extra}"
                                )
                            };

                            view! {
                                <table
                                    class="divide-y divide-gray-200"
                                    style="table-layout: fixed; width: 100%; \
                                           border-collapse: collapse;"
                                >
                                    <colgroup>
                                        // Checkbox column — minimal, not resizable.
                                        <col style="width: 36px;" />
                                        <col style=move || {
                                            format!("width: {}px;", name_w.get())
                                        } />
                                        <col style=move || {
                                            format!("width: {}px;", size_w.get())
                                        } />
                                        <col style=move || {
                                            format!("width: {}px;", modified_w.get())
                                        } />
                                    </colgroup>

                                    <thead class="sticky top-0 z-10">
                                        <tr>
                                            // Checkbox header — no label.
                                            <th class="px-1 py-2.5 bg-gray-50 \
                                                       border-r border-gray-200" />

                                            // Name ─────────────────────────────
                                            <th
                                                class=hdr_cls(SortCol::Name, "text-left")
                                                on:click=move |_| toggle_sort(SortCol::Name)
                                            >
                                                {name_label}
                                                <div
                                                    class="absolute inset-y-0 right-0 w-px \
                                                           bg-gray-200 hover:w-1 \
                                                           hover:bg-gray-400 \
                                                           cursor-col-resize transition-all z-10"
                                                    on:mousedown=move |e: web_sys::MouseEvent| {
                                                        e.stop_propagation();
                                                        start_resize(name_w, e);
                                                    }
                                                    on:click=move |e: web_sys::MouseEvent| {
                                                        e.stop_propagation();
                                                    }
                                                />
                                            </th>

                                            // Size ─────────────────────────────
                                            <th
                                                class=hdr_cls(SortCol::Size, "text-right")
                                                on:click=move |_| toggle_sort(SortCol::Size)
                                            >
                                                {size_label}
                                                <div
                                                    class="absolute inset-y-0 right-0 w-px \
                                                           bg-gray-200 hover:w-1 \
                                                           hover:bg-gray-400 \
                                                           cursor-col-resize transition-all z-10"
                                                    on:mousedown=move |e: web_sys::MouseEvent| {
                                                        e.stop_propagation();
                                                        start_resize(size_w, e);
                                                    }
                                                    on:click=move |e: web_sys::MouseEvent| {
                                                        e.stop_propagation();
                                                    }
                                                />
                                            </th>

                                            // Modified — no resize handle on last col.
                                            <th
                                                class=hdr_cls(SortCol::Modified, "text-right")
                                                on:click=move |_| toggle_sort(SortCol::Modified)
                                            >
                                                {modified_label}
                                            </th>
                                        </tr>
                                    </thead>

                                    <tbody class="divide-y divide-gray-100">
                                        // Up-one-level row.
                                        {move || {
                                            let path = current_path.get();
                                            if !path.is_root() {
                                                let parent = path.parent().unwrap_or_else(|| {
                                                    CatalogPath::new("/")
                                                        .expect("root always valid")
                                                });
                                                view! {
                                                    <tr
                                                        class="hover:bg-gray-50 cursor-pointer \
                                                               select-none"
                                                        on:click=move |_| {
                                                            selected.set(Vec::new());
                                                            current_path.set(parent.clone());
                                                        }
                                                    >
                                                        <td class="px-1 py-2.5 \
                                                                   border-r border-gray-100" />
                                                        <td class="px-3 py-2.5 text-sm \
                                                                   text-gray-400 italic \
                                                                   border-r border-gray-100 \
                                                                   overflow-hidden \
                                                                   text-ellipsis whitespace-nowrap">
                                                            <span class="flex items-center gap-2">
                                                                <span
                                                                    class="material-symbols-outlined \
                                                                           text-gray-300"
                                                                    style="font-size:18px;"
                                                                >
                                                                    "arrow_upward"
                                                                </span>
                                                                ".. (parent)"
                                                            </span>
                                                        </td>
                                                        <td class="px-3 py-2.5 \
                                                                   border-r border-gray-100" />
                                                        <td class="px-3 py-2.5" />
                                                    </tr>
                                                }
                                                .into_any()
                                            } else {
                                                view! { <tr></tr> }.into_any()
                                            }
                                        }}

                                        // Folder rows.
                                        {folders
                                            .into_iter()
                                            .map(|folder| {
                                                let name = folder.path.name().to_owned();
                                                let modified = short_date(&folder.modified_at);
                                                let folder_path = folder.path.clone();
                                                let item = SelectedItem {
                                                    path: folder.path.clone(),
                                                    kind: ItemKind::Folder,
                                                };
                                                let item2 = item.clone();
                                                view! {
                                                    <tr class="hover:bg-gray-50 select-none">
                                                        <td class="px-1 py-2.5 text-center \
                                                                   border-r border-gray-100">
                                                            <input
                                                                type="checkbox"
                                                                class="rounded border-gray-300"
                                                                prop:checked=move || {
                                                                    selected
                                                                        .get()
                                                                        .contains(&item2)
                                                                }
                                                                on:change=move |_| {
                                                                    toggle_select(item.clone())
                                                                }
                                                            />
                                                        </td>
                                                        <td
                                                            class="px-3 py-2.5 cursor-pointer \
                                                                   border-r border-gray-100 \
                                                                   overflow-hidden \
                                                                   text-ellipsis whitespace-nowrap"
                                                            on:click=move |_| {
                                                                selected.set(Vec::new());
                                                                current_path
                                                                    .set(folder_path.clone());
                                                            }
                                                        >
                                                            <span
                                                                class="flex items-center gap-2 \
                                                                       text-sm font-medium \
                                                                       text-gray-800"
                                                            >
                                                                <span
                                                                    class="material-symbols-outlined \
                                                                           text-gray-400"
                                                                    style="font-size:18px;"
                                                                >
                                                                    "folder"
                                                                </span>
                                                                {name}
                                                            </span>
                                                        </td>
                                                        <td class="px-3 py-2.5 text-right \
                                                                   text-sm text-gray-300 \
                                                                   border-r border-gray-100">
                                                            "—"
                                                        </td>
                                                        <td class="px-3 py-2.5 text-right \
                                                                   text-sm text-gray-400 \
                                                                   tabular-nums">
                                                            {modified}
                                                        </td>
                                                    </tr>
                                                }
                                            })
                                            .collect_view()}

                                        // File rows.
                                        // Click is reserved for future preview; download is
                                        // triggered from the dedicated toolbar button.
                                        {files
                                            .into_iter()
                                            .map(|file| {
                                                let name = file.path.name().to_owned();
                                                let size = format_size(file.size_bytes);
                                                let modified = short_date(&file.modified_at);
                                                let icon = file_icon(&file.content_type);
                                                let item = SelectedItem {
                                                    path: file.path.clone(),
                                                    kind: ItemKind::File,
                                                };
                                                let item2 = item.clone();
                                                view! {
                                                    <tr class="hover:bg-gray-50 select-none">
                                                        <td class="px-1 py-2.5 text-center \
                                                                   border-r border-gray-100">
                                                            <input
                                                                type="checkbox"
                                                                class="rounded border-gray-300"
                                                                prop:checked=move || {
                                                                    selected
                                                                        .get()
                                                                        .contains(&item2)
                                                                }
                                                                on:change=move |_| {
                                                                    toggle_select(item.clone())
                                                                }
                                                            />
                                                        </td>
                                                        <td class="px-3 py-2.5 \
                                                                   border-r border-gray-100 \
                                                                   overflow-hidden \
                                                                   text-ellipsis whitespace-nowrap">
                                                            <span
                                                                class="flex items-center gap-2 \
                                                                       text-sm text-gray-800"
                                                            >
                                                                <span
                                                                    class="material-symbols-outlined \
                                                                           text-gray-400"
                                                                    style="font-size:18px;"
                                                                >
                                                                    {icon}
                                                                </span>
                                                                {name}
                                                            </span>
                                                        </td>
                                                        <td class="px-3 py-2.5 text-right \
                                                                   text-sm text-gray-500 \
                                                                   border-r border-gray-100 \
                                                                   tabular-nums">
                                                            {size}
                                                        </td>
                                                        <td class="px-3 py-2.5 text-right \
                                                                   text-sm text-gray-400 \
                                                                   tabular-nums">
                                                            {modified}
                                                        </td>
                                                    </tr>
                                                }
                                            })
                                            .collect_view()}
                                    </tbody>
                                </table>
                            }
                            .into_any()
                        }
                    }
                }}
            </div>

            // ── Stats footer ──────────────────────────────────────────────────
            // Pinned to the bottom; only rendered once data is available.
            {move || match contents.map(|r| r.clone()) {
                Some(Ok(data)) => {
                    let nf = data.folders.len();
                    let nfiles = data.files.len();
                    view! {
                        <div class="flex-shrink-0 border-t border-gray-100 px-4 py-1.5 \
                                     flex items-center gap-4 text-xs text-gray-400">
                            <span>
                                <strong class="text-gray-500 tabular-nums">{nf}</strong>
                                {if nf == 1 { " subfolder" } else { " subfolders" }}
                            </span>
                            <span>
                                <strong class="text-gray-500 tabular-nums">{nfiles}</strong>
                                {if nfiles == 1 { " file" } else { " files" }}
                            </span>
                        </div>
                    }
                    .into_any()
                }
                _ => view! { <div></div> }.into_any(),
            }}
        </div>
    }
}
