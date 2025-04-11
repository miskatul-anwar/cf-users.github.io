#![allow(non_snake_case)]
use ::leptos::task::spawn_local;
use leptos::prelude::*;
use reqwest;
use serde::Deserialize;
use thaw::*;

#[derive(Debug, Clone, Deserialize, Default)]
struct ApiResponse<T> {
    status: String,
    result: Vec<T>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct User {
    handle: String,
    rating: Option<i32>,
    maxRating: Option<i32>,
    rank: Option<String>,
    country: Option<String>,
    titlePhoto: Option<String>,
    organization: Option<String>,
    contribution: Option<i32>,
}

// Helper function to assign rank-based colors
fn get_rank_color(rank: &str) -> &'static str {
    match rank.to_lowercase().as_str() {
        "newbie" => "gray",                       // 0-1199
        "pupil" => "green",                       // 1200-1399
        "specialist" => "cyan",                   // 1400-1599
        "expert" => "blue",                       // 1600-1899
        "candidate master" => "violet",           // 1900-2099
        "master" => "lightorange",                // 2100-2299
        "international master" => "orange",       // 2300-2499
        "grandmaster" => "brightred",             // 2500-2599
        "international grandmaster" => "darkred", // 2600-2999
        "legendary grandmaster" => "darkred",     // 3000+
        _ => "black",                             // Default for unknown ranks
    }
}

#[component]
fn App() -> impl IntoView {
    let username = RwSignal::new(String::from(""));
    let user_data = RwSignal::new(User::default());

    let fetch_codeforces_user = move |_| {
        let name = username.get();
        spawn_local(async move {
            let url = format!("https://codeforces.com/api/user.info?handles={}", name);
            match reqwest::get(&url).await {
                Ok(resp) => match resp.json::<ApiResponse<User>>().await {
                    Ok(data) if data.status == "OK" => {
                        if let Some(user) = data.result.first().cloned() {
                            user_data.set(user);
                        }
                    }
                    _ => user_data.set(User::default()), // Reset to default if response fails
                },
                Err(_) => user_data.set(User::default()),
            }
        });
    };

    let display_user_info = move || {
        let user = user_data.get();
        let rank = user.rank.clone().unwrap_or("".into());
        let rank_color = get_rank_color(&rank);

        view! {
            <Layout has_sider=true>
                <LayoutSider>
                    <img
                        style="width: 100%; display: block; margin: auto;"
                        src=user.titlePhoto.unwrap_or(String::from(""))
                    />
                </LayoutSider>
                <Layout>
                    <LayoutHeader attr:style=" padding: 20px;">
                        <p style="font-size:1.5em;">{user.handle.clone()}</p>
                        <p style=format!("color: {};", rank_color)>{rank}</p>
                    </LayoutHeader>
                    <Layout attr:style=" padding: 20px;">
                        <>
                            <p>
                                <b>"Contest Rating: "</b>
                                {user.rating.unwrap_or(0)}
                                "(Max:"
                                {user.maxRating.unwrap_or(0)}
                                ")"
                            </p>
                            <p>
                                <b>"Contribution: "</b>
                                "+"
                                {user.contribution.unwrap_or(0)}
                            </p>
                            <p>
                                <b>"Organization: "</b>
                                {user.organization.clone().unwrap_or("N/A".into())}
                            </p>
                            <p>
                                <b>"Country: "</b>
                                {user.country.clone().unwrap_or("N/A".into())}
                            </p>
                        </>

                    </Layout>
                </Layout>
            </Layout>
        }
    };

    view! {
        <ConfigProvider>
            <Card>
                <CardPreview>
                    <img
                        src="https://codeforces.org/s/85604/images/codeforces-sponsored-by-ton.png"
                        style="width: 50%; display: block; margin: auto;"
                    />
                </CardPreview>

                <div style="max-width: fit-content; margin: auto;">
                    <Flex gap=FlexGap::Large align=FlexAlign::Center>
                        <Input
                            placeholder="Enter username"
                            on:input=move |ev| username.set(event_target_value(&ev))
                        >
                            <InputPrefix slot>
                                <Icon icon=icondata::AiUserOutlined />
                            </InputPrefix>
                        </Input>
                        <Button appearance=ButtonAppearance::Primary on:click=fetch_codeforces_user>
                            "Search"
                        </Button>
                    </Flex>
                </div>

                <Card>
                    <CardHeader>
                        <div style="max-width: fit-content; margin-left: auto; margin-right:auto">
                            <h2>"User Info"</h2>
                        </div>
                    </CardHeader>
                    <div style="max-width: fit-content; margin-left: auto; margin-right:auto;">
                        {display_user_info}
                    </div>
                </Card>
            </Card>
        </ConfigProvider>
    }
}

fn main() {
    mount_to_body(App);
}
