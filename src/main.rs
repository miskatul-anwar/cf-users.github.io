#![allow(non_snake_case)]

mod api;
mod components;
mod storage;
mod store;
mod util;
mod views;

use components::{CopyLinkButton, ThemeToggle};
use leptos::prelude::*;
use store::Route;
use thaw::*;

use views::compare_view::CompareView;
use views::contests_view::ContestsView;
use views::problems_view::ProblemsView;
use views::recent_view::RecentView;
use views::user_view::UserView;

fn tab_key(route: &Route) -> &'static str {
    match route {
        Route::User(_) => "user",
        Route::Compare(_) => "compare",
        Route::Contests => "contests",
        Route::Problems => "problems",
        Route::Recent => "recent",
    }
}

fn route_for_tab(key: &str) -> Route {
    match key {
        "compare" => Route::Compare(Vec::new()),
        "contests" => Route::Contests,
        "problems" => Route::Problems,
        "recent" => Route::Recent,
        _ => Route::User(None),
    }
}

#[component]
fn App() -> impl IntoView {
    let route = store::route();
    let theme = store::theme();

    // One tab signal kept in sync with the router in both directions.
    let tab = RwSignal::new(tab_key(&route.get_untracked()).to_string());

    // Router -> tabs (deep links, back button).
    Effect::new(move |_| {
        let key = tab_key(&route.get()).to_string();
        if tab.get_untracked() != key {
            tab.set(key);
        }
    });

    // Tabs -> router (clicks).
    Effect::new(move |_| {
        let key = tab.get();
        if tab_key(&route.get_untracked()) != key.as_str() {
            store::go(route_for_tab(&key));
        }
    });

    // Body background follows the theme; remove the loading splash on first paint.
    Effect::new(move |_| {
        theme.track();
        let dark = store::is_dark();
        if let Some(body) = document().body() {
            let _ = body
                .style()
                .set_property("background-color", if dark { "#1b1a19" } else { "#faf9f8" });
            let _ = body
                .style()
                .set_property("color", if dark { "#f3f2f1" } else { "#242424" });
        }
        storage::remove_element("cfx-splash");
    });

    view! {
        <ConfigProvider theme=theme>
            <div style="max-width:1150px;margin:0 auto;padding:10px 16px 56px;">
                <header style="display:flex;align-items:center;justify-content:space-between;gap:14px;flex-wrap:wrap;margin-bottom:2px;">
                    <h1
                        title="Codeforces Explorer"
                        style="margin:0;font-size:1.65em;cursor:pointer;"
                        on:click=move |_| store::go(Route::User(None))
                    >
                        <span style="background:linear-gradient(90deg,#0078d4,#aa00aa);-webkit-background-clip:text;background-clip:text;color:transparent;font-weight:800;">
                            "Codeforces"
                        </span>
                        <span style="font-weight:300;">" Explorer"</span>
                    </h1>
                    <Flex gap=FlexGap::Medium align=FlexAlign::Center>
                        <CopyLinkButton text={window().location().href().unwrap_or_default()}/>
                        <ThemeToggle/>
                        <a
                            href="https://github.com/miskatul-anwar/cf-users.github.io"
                            target="_blank"
                            rel="noopener"
                            style="text-decoration:none;color:inherit;font-weight:600;font-size:0.85rem;opacity:.75;"
                        >
                            "GitHub \u{2197}"
                        </a>
                    </Flex>
                </header>
                <p style="text-align:center;color:#888;margin:0 0 12px;">
                    "Profiles, analytics, comparisons, contests, problems and the live feed \u{2014} powered by the Codeforces API."
                </p>

                <TabList selected_value=tab>
                    <Tab value="user">"User"</Tab>
                    <Tab value="compare">"Compare"</Tab>
                    <Tab value="contests">"Contests"</Tab>
                    <Tab value="problems">"Problems"</Tab>
                    <Tab value="recent">"Recent"</Tab>
                </TabList>

                <main style="margin-top:10px;">
                    {move || {
                        match route.get() {
                            Route::Compare(handles) => {
                                let mut it = handles.into_iter();
                                view! {
                                    <CompareView initial_a=it.next() initial_b=it.next()/>
                                }.into_any()
                            }
                            Route::User(h) => view! { <UserView initial_handle=h/> }.into_any(),
                            Route::Contests => view! { <ContestsView/> }.into_any(),
                            Route::Problems => view! { <ProblemsView/> }.into_any(),
                            Route::Recent => view! { <RecentView/> }.into_any(),
                        }
                    }}
                </main>

                <footer style="text-align:center;color:#aaa;margin-top:36px;">
                    <Caption1>
                        "Data from "
                        <a href="https://codeforces.com/api/help" target="_blank" rel="noopener">
                            "Codeforces API"
                        </a>
                        " \u{00b7} cached locally for speed \u{00b7} not affiliated with Codeforces"
                    </Caption1>
                </footer>
            </div>
        </ConfigProvider>
    }
}

fn main() {
    // Surface Rust panics in the browser console (message + source location)
    // instead of an opaque `RuntimeError: unreachable` wasm trap.
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    mount_to_body(App);
}
