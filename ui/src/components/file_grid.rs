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

fn file_icon(content_type: &str) -> &'static str {
    if content_type.starts_with("image/") {
        "🖼"
    } else if content_type.starts_with("video/") {
        "🎬"
    } else if content_type.starts_with("audio/") {
        "🎵"
    } else if content_type.contains("pdf") {
        "📄"
    } else if content_type.contains("zip")
        || content_type.contains("tar")
        || content_type.contains("gz")
    {
        "🗜"
    } else {
        "📄"
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
    // Which column signal is being dragged, and where the drag started.

    let drag_col: RwSignal<Option<RwSignal<f64>>> = RwSignal::new(None);
    let drag_x0: RwSignal<f64> = RwSignal::new(0.0);
    let drag_w0: RwSignal<f64> = RwSignal::new(0.0);

    // Persistent window-level mousemove / mouseup listeners.
    // Closure::forget is intentional: FileGrid is mounted for the entire app
    // lifetime, so these listeners never need to be removed.
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

    // Starts a resize drag for the given column width signal.
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

    view! {
        // Transparent full-viewport overlay while dragging.
        // Keeps the col-resize cursor and captures all mouse events so dragging
        // doesn't feel "sticky" when the pointer moves fast.
        <Show when=move || drag_col.get().is_some()>
            <div class="fixed inset-0 z-50 cursor-col-resize" />
        </Show>

        <div class="bg-white rounded-lg shadow-sm border border-gray-200 overflow-x-auto">
            {move || {
                // Reading sort signals here makes this closure react to sort
                // changes as well as to contents changes.
                let col = sort_col.get();
                let dir = sort_dir.get();

                let resolved = contents.map(|r| r.clone());
                match resolved {
                    None => view! {
                        <div class="p-10 text-center text-gray-400">
                            <div class="text-4xl mb-2">"⏳"</div>
                            "Loading..."
                        </div>
                    }
                    .into_any(),

                    Some(Err(ref e)) => {
                        let msg = e.to_string();
                        view! {
                            <div class="p-10 text-center text-red-600">
                                <div class="text-4xl mb-2">"⚠"</div>
                                {msg}
                            </div>
                        }
                        .into_any()
                    }

                    Some(Ok(data)) => {
                        if data.folders.is_empty() && data.files.is_empty() {
                            return view! {
                                <div class="p-10 text-center text-gray-400">
                                    <div class="text-4xl mb-2">"📭"</div>
                                    "This folder is empty"
                                </div>
                            }
                            .into_any();
                        }

                        // Sort folders (no size column → fall back to name).
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

                        // Sort files.
                        let mut files = data.files.clone();
                        files.sort_by(|a, b| {
                            let ord = match col {
                                SortCol::Name => a.path.name().cmp(b.path.name()),
                                SortCol::Size => a.size_bytes.cmp(&b.size_bytes),
                                SortCol::Modified => a.modified_at.cmp(&b.modified_at),
                            };
                            if dir == SortDir::Desc { ord.reverse() } else { ord }
                        });

                        // Header label: append sort arrow for the active column.
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

                        // Header cell base class; active column gets blue text.
                        let hdr_cls = |c: SortCol, extra: &'static str| -> String {
                            let color = if c == col {
                                "text-blue-600 font-semibold"
                            } else {
                                "text-gray-500 font-medium"
                            };
                            format!(
                                "relative px-3 py-3 text-xs uppercase tracking-wider \
                                 cursor-pointer select-none bg-gray-50 \
                                 border-r border-gray-200 {color} {extra}"
                            )
                        };

                        view! {
                            // table-layout:fixed + explicit col widths allow JS-driven resizing.
                            <table
                                class="divide-y divide-gray-200"
                                style="table-layout: fixed; width: 100%; border-collapse: collapse;"
                            >
                                <colgroup>
                                    // Checkbox column — fixed 40 px, not resizable.
                                    <col style="width: 40px;" />
                                    // Resizable columns track their signal-driven widths.
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

                                <thead>
                                    <tr>
                                        // Checkbox column header.
                                        <th class="px-3 py-3 bg-gray-50 border-r border-gray-200" />

                                        // Name ─────────────────────────────
                                        <th
                                            class=hdr_cls(SortCol::Name, "text-left")
                                            on:click=move |_| toggle_sort(SortCol::Name)
                                        >
                                            {name_label}
                                            // Resize handle — visible 1 px divider, expands on hover.
                                            <div
                                                class="absolute inset-y-0 right-0 w-px \
                                                       bg-gray-300 hover:w-1 hover:bg-blue-400 \
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
                                                       bg-gray-300 hover:w-1 hover:bg-blue-400 \
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

                                        // Modified ─────────────────────────
                                        // Last column: no resize handle (fills remaining space).
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
                                            let parent = path
                                                .parent()
                                                .unwrap_or_else(|| {
                                                    CatalogPath::new("/")
                                                        .expect("root always valid")
                                                });
                                            view! {
                                                <tr
                                                    class="hover:bg-blue-50 cursor-pointer select-none"
                                                    on:click=move |_| {
                                                        selected.set(Vec::new());
                                                        current_path.set(parent.clone());
                                                    }
                                                >
                                                    <td class="px-3 py-3 border-r border-gray-100" />
                                                    <td class="px-3 py-3 text-sm text-gray-500 italic \
                                                               border-r border-gray-100 overflow-hidden \
                                                               text-ellipsis whitespace-nowrap">
                                                        <span class="flex items-center gap-2">
                                                            <span class="text-xl">"📁"</span>
                                                            ".. (parent)"
                                                        </span>
                                                    </td>
                                                    <td class="px-3 py-3 border-r border-gray-100" />
                                                    <td class="px-3 py-3" />
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
                                                    <td class="px-3 py-3 border-r border-gray-100">
                                                        <input
                                                            type="checkbox"
                                                            class="rounded border-gray-300"
                                                            prop:checked=move || {
                                                                selected.get().contains(&item2)
                                                            }
                                                            on:change=move |_| {
                                                                toggle_select(item.clone())
                                                            }
                                                        />
                                                    </td>
                                                    <td
                                                        class="px-3 py-3 cursor-pointer \
                                                               border-r border-gray-100 \
                                                               overflow-hidden text-ellipsis whitespace-nowrap"
                                                        on:click=move |_| {
                                                            selected.set(Vec::new());
                                                            current_path.set(folder_path.clone());
                                                        }
                                                    >
                                                        <span class="flex items-center gap-2 \
                                                                     text-sm font-medium text-gray-800">
                                                            <span class="text-xl">"📁"</span>
                                                            {name}
                                                        </span>
                                                    </td>
                                                    <td class="px-3 py-3 text-right text-sm \
                                                               text-gray-400 border-r border-gray-100">
                                                        "—"
                                                    </td>
                                                    <td class="px-3 py-3 text-right text-sm text-gray-400">
                                                        {modified}
                                                    </td>
                                                </tr>
                                            }
                                        })
                                        .collect_view()}

                                    // File rows.
                                    {files
                                        .into_iter()
                                        .map(|file| {
                                            let name = file.path.name().to_owned();
                                            let size = format_size(file.size_bytes);
                                            let modified = short_date(&file.modified_at);
                                            let icon = file_icon(&file.content_type).to_owned();
                                            let file_path = file.path.clone();
                                            let item = SelectedItem {
                                                path: file.path.clone(),
                                                kind: ItemKind::File,
                                            };
                                            let item2 = item.clone();
                                            view! {
                                                <tr class="hover:bg-gray-50 select-none">
                                                    <td class="px-3 py-3 border-r border-gray-100">
                                                        <input
                                                            type="checkbox"
                                                            class="rounded border-gray-300"
                                                            prop:checked=move || {
                                                                selected.get().contains(&item2)
                                                            }
                                                            on:change=move |_| {
                                                                toggle_select(item.clone())
                                                            }
                                                        />
                                                    </td>
                                                    <td class="px-3 py-3 border-r border-gray-100 \
                                                               overflow-hidden text-ellipsis whitespace-nowrap">
                                                        <a
                                                            href=format!(
                                                                "http://localhost:3000/files/{}",
                                                                file_path
                                                                    .as_str()
                                                                    .trim_start_matches('/')
                                                            )
                                                            target="_blank"
                                                            class="flex items-center gap-2 text-sm \
                                                                   text-gray-800 hover:text-blue-600"
                                                        >
                                                            <span class="text-xl">{icon}</span>
                                                            {name}
                                                        </a>
                                                    </td>
                                                    <td class="px-3 py-3 text-right text-sm \
                                                               text-gray-500 border-r border-gray-100">
                                                        {size}
                                                    </td>
                                                    <td class="px-3 py-3 text-right text-sm text-gray-400">
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
    }
}
