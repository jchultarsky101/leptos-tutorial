use common::dto::{DayCount, StatsDto};
use leptos::prelude::*;

use crate::{api, error::UiError};

// ── Byte formatting (reused from file_grid logic) ─────────────────────────────

fn fmt_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024u64 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

// ── SVG bar chart ─────────────────────────────────────────────────────────────

/// Renders an SVG bar chart of daily upload counts.
/// Shows the most recent `MAX_DAYS` days to keep the chart readable.
const MAX_DAYS: usize = 14;
const CHART_W: f64 = 440.0;
const CHART_H: f64 = 120.0;
const AXIS_X: f64 = 0.0;
const AXIS_Y: f64 = CHART_H - 20.0; // leave 20 px for labels

#[component]
fn UploadsChart(days: Vec<DayCount>) -> impl IntoView {
    // Take the last MAX_DAYS entries (already sorted ascending from the API).
    let days: Vec<DayCount> = days.into_iter().rev().take(MAX_DAYS).rev().collect();
    let n = days.len();

    if n == 0 {
        return view! {
            <p class="text-sm text-gray-400 text-center py-4">"No uploads recorded yet."</p>
        }
        .into_any();
    }

    let max_count = days.iter().map(|d| d.count).max().unwrap_or(1).max(1);
    let bar_area_w = CHART_W - AXIS_X;
    let slot_w = bar_area_w / n as f64;
    let bar_w = (slot_w * 0.6).max(4.0);
    let bar_area_h = AXIS_Y - 4.0; // 4 px top padding

    let bars: Vec<_> = days
        .iter()
        .enumerate()
        .map(|(i, day)| {
            let cx = AXIS_X + slot_w * i as f64 + slot_w / 2.0;
            let bar_h = (day.count as f64 / max_count as f64) * bar_area_h;
            let bar_y = AXIS_Y - bar_h;
            // Label: "MM-DD" from "YYYY-MM-DD"
            let label = day.date.get(5..).unwrap_or(day.date.as_str()).to_owned();
            let count = day.count;
            (cx, bar_y, bar_h, bar_w, label, count)
        })
        .collect();

    view! {
        <svg
            viewBox=format!("0 0 {} {}", CHART_W, CHART_H)
            class="w-full"
            style=format!("height: {}px;", CHART_H)
            aria-label="Uploads per day bar chart"
        >
            // X-axis baseline
            <line
                x1=AXIS_X.to_string() y1=AXIS_Y.to_string()
                x2=CHART_W.to_string() y2=AXIS_Y.to_string()
                stroke="#e5e7eb" stroke-width="1"
            />

            // Bars + labels
            {bars.into_iter().map(|(cx, bar_y, bar_h, bw, label, count)| {
                let x = cx - bw / 2.0;
                view! {
                    <g>
                        // Bar
                        <rect
                            x=x.to_string()
                            y=bar_y.to_string()
                            width=bw.to_string()
                            height=bar_h.to_string()
                            rx="2"
                            fill="#3b82f6"
                            opacity="0.85"
                        />
                        // Count label above bar (hidden when bar is tiny)
                        {(bar_h > 14.0).then(|| view! {
                            <text
                                x=cx.to_string()
                                y=(bar_y - 3.0).to_string()
                                text-anchor="middle"
                                font-size="9"
                                fill="#6b7280"
                            >
                                {count.to_string()}
                            </text>
                        })}
                        // Date label below axis (rotated)
                        <text
                            x=cx.to_string()
                            y=(AXIS_Y + 14.0).to_string()
                            text-anchor="middle"
                            font-size="8"
                            fill="#9ca3af"
                        >
                            {label}
                        </text>
                    </g>
                }
            }).collect_view()}
        </svg>
    }
    .into_any()
}

// ── Stats content (after data loaded) ────────────────────────────────────────

#[component]
fn StatsContent(dto: StatsDto) -> impl IntoView {
    view! {
        // Summary tiles
        <div class="grid grid-cols-3 gap-3 mb-5">
            <div class="bg-blue-50 dark:bg-blue-900/30 rounded-lg px-3 py-3 text-center">
                <p class="text-2xl font-bold text-blue-700 dark:text-blue-300">{dto.total_files.to_string()}</p>
                <p class="text-xs text-blue-500 dark:text-blue-400 mt-0.5">"Files"</p>
            </div>
            <div class="bg-purple-50 dark:bg-purple-900/30 rounded-lg px-3 py-3 text-center">
                <p class="text-2xl font-bold text-purple-700 dark:text-purple-300">
                    {dto.total_folders.to_string()}
                </p>
                <p class="text-xs text-purple-500 dark:text-purple-400 mt-0.5">"Folders"</p>
            </div>
            <div class="bg-green-50 dark:bg-green-900/30 rounded-lg px-3 py-3 text-center">
                <p class="text-2xl font-bold text-green-700 dark:text-green-300">
                    {fmt_bytes(dto.total_size_bytes)}
                </p>
                <p class="text-xs text-green-500 dark:text-green-400 mt-0.5">"Total size"</p>
            </div>
        </div>

        // Upload history chart
        <p class="text-xs font-semibold text-gray-400 dark:text-gray-500 uppercase tracking-wider mb-2">
            "Uploads by day (last 14 days)"
        </p>
        <UploadsChart days=dto.uploads_by_day />
    }
}

// ── StatsModal ────────────────────────────────────────────────────────────────

#[component]
pub fn StatsModal() -> impl IntoView {
    let stats_open = use_context::<RwSignal<bool>>().expect("stats_open context");

    // Fetch stats every time the modal opens.
    let stats: LocalResource<Result<StatsDto, UiError>> =
        LocalResource::new(move || async move { api::get_stats().await });

    view! {
        <Show when=move || stats_open.get()>
            // Backdrop
            <div
                class="fixed inset-0 z-50 bg-black/50 flex items-center justify-center p-6"
                on:click=move |_| stats_open.set(false)
            >
                // Panel — stop propagation so clicks inside don't close it.
                <div
                    class="bg-white dark:bg-gray-800 rounded-xl shadow-2xl w-full max-w-lg overflow-hidden"
                    on:click=move |e: web_sys::MouseEvent| e.stop_propagation()
                >
                    // Header
                    <div class="flex items-center justify-between \
                                px-6 py-4 border-b border-gray-100 dark:border-gray-700">
                        <h2 class="text-base font-semibold text-gray-900 dark:text-white \
                                   flex items-center gap-2">
                            <span class="material-symbols-outlined text-blue-600"
                                style="font-size:20px;">"analytics"</span>
                            "Ember Trove Statistics"
                        </h2>
                        <button
                            class="text-gray-400 dark:text-gray-500 \
                                   hover:text-gray-600 dark:hover:text-gray-300 \
                                   transition-colors focus:outline-none"
                            on:click=move |_| stats_open.set(false)
                            aria-label="Close statistics"
                        >
                            <span class="material-symbols-outlined"
                                style="font-size:20px;">"close"</span>
                        </button>
                    </div>

                    // Body
                    <div class="px-6 py-5">
                        {move || {
                            match stats.map(|r| r.clone()) {
                                None => view! {
                                    <div class="flex flex-col items-center py-8 text-gray-400 dark:text-gray-500">
                                        <span class="material-symbols-outlined text-3xl">
                                            "hourglass_empty"
                                        </span>
                                        <p class="text-sm mt-1">"Loading…"</p>
                                    </div>
                                }
                                .into_any(),
                                Some(Err(e)) => view! {
                                    <p class="text-sm text-red-600">
                                        {format!("Failed to load statistics: {e}")}
                                    </p>
                                }
                                .into_any(),
                                Some(Ok(dto)) => view! {
                                    <StatsContent dto />
                                }
                                .into_any(),
                            }
                        }}
                    </div>
                </div>
            </div>
        </Show>
    }
}
