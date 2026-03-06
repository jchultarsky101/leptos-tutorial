use leptos::prelude::*;
use wasm_bindgen::prelude::*;

/// Clear the search input element's value via DOM.
pub(crate) fn clear_search_input() {
    if let Some(doc) = web_sys::window().and_then(|w| w.document())
        && let Ok(Some(el)) = doc.query_selector("#search-input")
        && let Some(input) = el.dyn_ref::<web_sys::HtmlInputElement>()
    {
        input.set_value("");
    }
}

/// Search bar placed between the header and breadcrumbs.
///
/// Reads `search_query` and `search_fuzzy` from context and updates them on
/// user input. Input is debounced (300 ms) to avoid excessive API calls.
#[component]
pub fn SearchBar() -> impl IntoView {
    let search_query: RwSignal<Option<String>> = use_context().expect("search_query context");
    let search_fuzzy: RwSignal<bool> = use_context().expect("search_fuzzy context");

    // Local text signal tracks the input field value before debounce fires.
    let input_text = RwSignal::new(String::new());
    // Stores the JS timeout handle so we can clear it on each keystroke.
    let timeout_handle: RwSignal<Option<i32>> = RwSignal::new(None);

    let on_input = move |ev: web_sys::Event| {
        let target = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok());
        let val = target.map(|t| t.value()).unwrap_or_default();
        input_text.set(val.clone());

        // Clear previous timeout.
        if let Some(handle) = timeout_handle.get_untracked() {
            let win = web_sys::window().expect("no window");
            win.clear_timeout_with_handle(handle);
        }

        // Debounce: set query after 300 ms.
        let cb = Closure::<dyn Fn()>::new(move || {
            let trimmed = input_text.get_untracked();
            if trimmed.trim().is_empty() {
                search_query.set(None);
            } else {
                search_query.set(Some(trimmed));
            }
        });
        let win = web_sys::window().expect("no window");
        if let Ok(id) = win
            .set_timeout_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), 300)
        {
            timeout_handle.set(Some(id));
        }
        cb.forget();
    };

    let on_fuzzy_change = move |_| {
        search_fuzzy.update(|f| *f = !*f);
        // Re-trigger search immediately if there is a query.
        let text = input_text.get_untracked();
        if !text.trim().is_empty() {
            search_query.set(Some(text));
        }
    };

    let clear_search = move |_| {
        input_text.set(String::new());
        search_query.set(None);
        clear_search_input();
    };

    view! {
        <div class="flex-shrink-0 bg-white dark:bg-gray-800 border-b \
                     border-gray-200 dark:border-gray-700 \
                     px-4 py-2 flex items-center gap-3">
            // Search icon
            <span class="material-symbols-outlined text-gray-400 dark:text-gray-500"
                  style="font-size:20px;">"search"</span>

            // Text input
            <input
                id="search-input"
                type="text"
                placeholder="Search files and folders..."
                class="flex-1 min-w-0 text-sm bg-transparent border-none \
                       outline-none placeholder-gray-400 dark:placeholder-gray-600 \
                       text-gray-700 dark:text-gray-200"
                on:input=on_input
            />

            // Clear button (shown when search is active)
            <Show when=move || search_query.get().is_some()>
                <button
                    class="text-gray-400 hover:text-gray-600 focus:outline-none"
                    on:click=clear_search
                >
                    <span class="material-symbols-outlined" style="font-size:18px;">
                        "close"
                    </span>
                </button>
            </Show>

            // Fuzzy checkbox
            <label class="flex items-center gap-1.5 text-xs text-gray-500 \
                          dark:text-gray-400 cursor-pointer select-none whitespace-nowrap">
                <input
                    type="checkbox"
                    class="accent-blue-500"
                    prop:checked=move || search_fuzzy.get()
                    on:change=on_fuzzy_change
                />
                "Fuzzy"
            </label>
        </div>
    }
}
