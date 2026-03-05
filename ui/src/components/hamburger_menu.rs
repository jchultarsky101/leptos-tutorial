use leptos::prelude::*;

use crate::app::{AppSettings, DateFormat, GlobalSortDir, SortField, Theme, storage_set};

// ── CSS helpers ───────────────────────────────────────────────────────────────

fn pill(active: bool) -> &'static str {
    if active {
        "flex-1 py-1 rounded text-xs font-medium transition-colors bg-blue-600 text-white"
    } else {
        "flex-1 py-1 rounded text-xs font-medium transition-colors \
         bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300 \
         hover:bg-gray-200 dark:hover:bg-gray-600"
    }
}

fn theme_btn(active: bool) -> &'static str {
    if active {
        "flex-1 flex items-center justify-center gap-1 py-1.5 rounded \
         text-sm font-medium transition-colors bg-gray-900 text-white"
    } else {
        "flex-1 flex items-center justify-center gap-1 py-1.5 rounded \
         text-sm font-medium transition-colors \
         bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300 \
         hover:bg-gray-200 dark:hover:bg-gray-600"
    }
}

// ── HamburgerMenu ─────────────────────────────────────────────────────────────

#[component]
pub fn HamburgerMenu() -> impl IntoView {
    let theme = use_context::<RwSignal<Theme>>().expect("theme context");
    let settings = use_context::<RwSignal<AppSettings>>().expect("settings context");
    let stats_open = use_context::<RwSignal<bool>>().expect("stats_open context");

    let open = RwSignal::new(false);

    view! {
        // Push the button to the far right of the header flex row.
        <div class="relative ml-auto">
            <button
                class="p-2 rounded text-gray-400 hover:text-white hover:bg-gray-700 \
                       focus:outline-none transition-colors"
                on:click=move |_| open.update(|v| *v = !*v)
                aria-label="Open menu"
                aria-expanded=move || open.get().to_string()
            >
                <span class="material-symbols-outlined">"menu"</span>
            </button>

            <Show when=move || open.get()>
                // Invisible full-screen backdrop — click closes the panel.
                <div
                    class="fixed inset-0 z-40"
                    on:click=move |_| open.set(false)
                />

                // Dropdown panel
                <div
                    class="absolute right-0 top-full mt-1 w-72 \
                           bg-white dark:bg-gray-800 \
                           border border-gray-200 dark:border-gray-700 \
                           rounded-lg shadow-xl z-50 overflow-hidden \
                           divide-y divide-gray-100 dark:divide-gray-700"
                >
                    // ── Theme ──────────────────────────────────────────────────
                    <div class="px-4 py-3">
                        <p class="text-xs font-semibold text-gray-400 uppercase \
                                  tracking-wider mb-2">"Theme"</p>
                        <div class="flex gap-2">
                            <button
                                class=move || theme_btn(theme.get() == Theme::Light)
                                on:click=move |_| {
                                    theme.set(Theme::Light);
                                    storage_set("theme", "light");
                                }
                            >
                                <span class="material-symbols-outlined"
                                    style="font-size:16px;">"light_mode"</span>
                                "Light"
                            </button>
                            <button
                                class=move || theme_btn(theme.get() == Theme::Dark)
                                on:click=move |_| {
                                    theme.set(Theme::Dark);
                                    storage_set("theme", "dark");
                                }
                            >
                                <span class="material-symbols-outlined"
                                    style="font-size:16px;">"dark_mode"</span>
                                "Dark"
                            </button>
                        </div>
                    </div>

                    // ── Default sort ────────────────────────────────────────────
                    <div class="px-4 py-3">
                        <p class="text-xs font-semibold text-gray-400 uppercase \
                                  tracking-wider mb-2">"Default Sort"</p>
                        // Sort field
                        <div class="flex gap-1 mb-1.5">
                            <button
                                class=move || pill(settings.get().sort_field == SortField::Name)
                                on:click=move |_| {
                                    settings.update(|s| s.sort_field = SortField::Name);
                                    storage_set("sort_field", "name");
                                }
                            >"Name"</button>
                            <button
                                class=move || pill(settings.get().sort_field == SortField::Date)
                                on:click=move |_| {
                                    settings.update(|s| s.sort_field = SortField::Date);
                                    storage_set("sort_field", "date");
                                }
                            >"Date"</button>
                            <button
                                class=move || pill(settings.get().sort_field == SortField::Size)
                                on:click=move |_| {
                                    settings.update(|s| s.sort_field = SortField::Size);
                                    storage_set("sort_field", "size");
                                }
                            >"Size"</button>
                        </div>
                        // Sort direction
                        <div class="flex gap-1">
                            <button
                                class=move || {
                                    pill(settings.get().sort_dir == GlobalSortDir::Asc)
                                }
                                on:click=move |_| {
                                    settings.update(|s| s.sort_dir = GlobalSortDir::Asc);
                                    storage_set("sort_dir", "asc");
                                }
                            >"↑ Asc"</button>
                            <button
                                class=move || {
                                    pill(settings.get().sort_dir == GlobalSortDir::Desc)
                                }
                                on:click=move |_| {
                                    settings.update(|s| s.sort_dir = GlobalSortDir::Desc);
                                    storage_set("sort_dir", "desc");
                                }
                            >"↓ Desc"</button>
                        </div>
                    </div>

                    // ── Date display ────────────────────────────────────────────
                    <div class="px-4 py-3">
                        <p class="text-xs font-semibold text-gray-400 uppercase \
                                  tracking-wider mb-2">"Date Display"</p>
                        <div class="flex gap-1">
                            <button
                                class=move || {
                                    pill(settings.get().date_format == DateFormat::Relative)
                                }
                                on:click=move |_| {
                                    settings.update(|s| s.date_format = DateFormat::Relative);
                                    storage_set("date_format", "relative");
                                }
                            >"Relative"</button>
                            <button
                                class=move || {
                                    pill(settings.get().date_format == DateFormat::Absolute)
                                }
                                on:click=move |_| {
                                    settings.update(|s| s.date_format = DateFormat::Absolute);
                                    storage_set("date_format", "absolute");
                                }
                            >"Absolute"</button>
                        </div>
                    </div>

                    // ── Preview auto-open ───────────────────────────────────────
                    <div class="px-4 py-3">
                        <div class="flex items-center justify-between">
                            <span class="text-sm text-gray-700 dark:text-gray-300">"Preview auto-open"</span>
                            <button
                                class=move || {
                                    format!(
                                        "relative w-10 h-5 rounded-full transition-colors \
                                         focus:outline-none {}",
                                        if settings.get().preview_auto_open {
                                            "bg-blue-600"
                                        } else {
                                            "bg-gray-300 dark:bg-gray-600"
                                        }
                                    )
                                }
                                role="switch"
                                aria-checked=move || {
                                    settings.get().preview_auto_open.to_string()
                                }
                                on:click=move |_| {
                                    let next = !settings.get_untracked().preview_auto_open;
                                    settings.update(|s| s.preview_auto_open = next);
                                    storage_set(
                                        "preview_auto_open",
                                        if next { "true" } else { "false" },
                                    );
                                }
                            >
                                <span
                                    class=move || {
                                        format!(
                                            "absolute top-0.5 h-4 w-4 rounded-full bg-white \
                                             shadow transition-transform {}",
                                            if settings.get().preview_auto_open {
                                                "translate-x-5"
                                            } else {
                                                "translate-x-0.5"
                                            }
                                        )
                                    }
                                />
                            </button>
                        </div>
                    </div>

                    // ── Statistics ──────────────────────────────────────────────
                    <div class="px-4 py-3">
                        <button
                            class="w-full flex items-center gap-2 px-2 py-1.5 rounded \
                                   text-sm text-gray-700 dark:text-gray-300 \
                                   hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                            on:click=move |_| {
                                stats_open.set(true);
                                open.set(false);
                            }
                        >
                            <span class="material-symbols-outlined"
                                style="font-size:18px;">"analytics"</span>
                            "Statistics"
                        </button>
                    </div>
                </div>
            </Show>
        </div>
    }
}
