#![allow(non_snake_case)]

mod api;
mod components;
mod util;
mod views;

use leptos::prelude::*;
use thaw::*;

use views::contests_view::ContestsView;
use views::problems_view::ProblemsView;
use views::recent_view::RecentView;
use views::user_view::UserView;

#[component]
fn App() -> impl IntoView {
    let tab = RwSignal::new(String::from("user"));

    view! {
        <ConfigProvider>
            <div style="max-width:1100px;margin:0 auto;padding:12px 16px 48px;">
                <header style="display:flex;align-items:center;justify-content:center;gap:14px;margin-bottom:6px;">
                    <h1 style="margin:0;font-size:1.7em;">
                        "Codeforces "
                        <span style="color:#aa00aa;">"Explorer"</span>
                    </h1>
                </header>
                <p style="text-align:center;color:#888;margin:0 0 14px;">
                    "Search users, browse contests, standings, problems, hacks, blogs and more \u{2014} powered by the Codeforces API."
                </p>

                <TabList selected_value=tab>
                    <Tab value="user">"User"</Tab>
                    <Tab value="contests">"Contests"</Tab>
                    <Tab value="problems">"Problems"</Tab>
                    <Tab value="recent">"Recent"</Tab>
                </TabList>

                <main style="margin-top:10px;">
                    {move || match tab.get().as_str() {
                        "contests" => view! { <ContestsView/> }.into_any(),
                        "problems" => view! { <ProblemsView/> }.into_any(),
                        "recent" => view! { <RecentView/> }.into_any(),
                        _ => view! { <UserView/> }.into_any(),
                    }}
                </main>

                <footer style="text-align:center;color:#aaa;margin-top:32px;">
                    <Caption1>
                        "Data from "
                        <a href="https://codeforces.com/api/help" target="_blank" rel="noopener">"Codeforces API"</a>
                        ". Unauthenticated endpoints are supported."
                    </Caption1>
                </footer>
            </div>
        </ConfigProvider>
    }
}

fn main() {
    mount_to_body(App);
}
