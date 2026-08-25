//! Recent tab: global recent actions feed + the full rated list.

use crate::api;
use crate::components::*;
use crate::storage;
use crate::util::*;
use leptos::either::Either;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

const PAGE_SIZE: usize = 30;

#[component]
pub fn RecentView() -> impl IntoView {
    view! {
        <Flex vertical=true gap=FlexGap::Medium>
            <SectionHeader title="Recent actions".into()/>
            <RecentActions/>
            <SectionHeader title="Rated list".into()/>
            <RatedList/>
        </Flex>
    }
}

/// Small ★-prefixed vote chip, hidden for zero ratings.
fn star_chip(rating: i32) -> impl IntoView {
    if rating == 0 {
        Either::Left(())
    } else {
        let text = format!("\u{2605} {rating:+}");
        Either::Right(view! {
            <span style="color:#b8860b;font-size:0.78rem;font-weight:700;flex-shrink:0;">{text}</span>
        })
    }
}

#[component]
fn RecentActions() -> impl IntoView {
    let count = RwSignal::new(String::from("50"));
    let data = RwSignal::new(Vec::<api::RecentAction>::new());
    let loading = RwSignal::new(false);
    let error = RwSignal::new(String::new());

    Effect::new(move |_| {
        let max_count: u32 = count.get().parse().unwrap_or(50);
        loading.set(true);
        error.set(String::new());
        spawn_local(async move {
            match api::recent_actions_cached(max_count).await {
                Ok(a) => data.set(a),
                Err(e) => error.set(e),
            }
            loading.set(false);
        });
    });

    view! {
        <Flex gap=FlexGap::Small align=FlexAlign::Center>
            <Text>"Show:"</Text>
            <Select default_value="50" value=count>
                <option value="25">"25"</option>
                <option value="50">"50"</option>
                <option value="100">"100"</option>
                <option value="200">"200"</option>
            </Select>
        </Flex>
        {move || -> AnyView {
            if loading.get() {
                view! { <Loading label="Loading recent actions".into()/> }.into_any()
            } else if !error.get().is_empty() {
                view! { <ErrorBar message=error/> }.into_any()
            } else {
                let now = storage::now_secs();
                let items = data.get_untracked().into_iter().filter_map(|a| {
                    let when = rel_time(a.time_seconds, now);
                    if let Some(b) = a.blog_entry {
                        let title = truncate(&b.title, 90);
                        let who = b.author_handle;
                        let link = format!("https://codeforces.com/blog/entry/{}", b.id);
                        let r = b.rating.unwrap_or(0);
                        Some(view! {
                            <div style="padding:10px 14px;margin-bottom:8px;background:rgba(128,128,128,0.06);border:1px solid rgba(128,128,128,0.25);border-left:3px solid #0078d4;border-radius:6px;">
                                <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center>
                                    <Caption1>"Blog"</Caption1>
                                    {star_chip(r)}
                                </Flex>
                                <p style="margin:6px 0 4px;">
                                    <a href=link target="_blank" rel="noopener" style="font-weight:700;text-decoration:none;">{title}</a>
                                </p>
                                <Caption1><HandleLink handle=who/>" \u{00b7} "{when}</Caption1>
                            </div>
                        }.into_any())
                    } else {
                        let c = a.comment?;
                        let text = truncate(&strip_html(&c.text), 140);
                        let who = c.commentator_handle;
                        let link = format!("https://codeforces.com/blog/entry/{}", c.entry_id);
                        let r = c.rating.unwrap_or(0);
                        Some(view! {
                            <div style="padding:10px 14px;margin-bottom:8px;background:rgba(128,128,128,0.06);border:1px solid rgba(128,128,128,0.25);border-left:3px solid #aa00aa;border-radius:6px;">
                                <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center>
                                    <Caption1>"Comment"</Caption1>
                                    {star_chip(r)}
                                </Flex>
                                <p style="margin:6px 0 4px;">
                                    <a href=link target="_blank" rel="noopener" style="font-weight:700;text-decoration:none;">{text}</a>
                                </p>
                                <Caption1><HandleLink handle=who/>" \u{00b7} "{when}</Caption1>
                            </div>
                        }.into_any())
                    }
                }).collect_view();
                view! { <div>{items}</div> }.into_any()
            }
        }}
    }
}

/// CSV export of the fully filtered rated list (capped at 10k rows).
fn rated_list_csv(users: &[api::User]) -> String {
    let mut rows = vec![vec![
        "Pos".into(),
        "Handle".into(),
        "Rank".into(),
        "Rating".into(),
        "MaxRating".into(),
        "Contribution".into(),
        "Organization".into(),
        "Country".into(),
    ]];
    for (i, u) in users.iter().take(10_000).enumerate() {
        rows.push(vec![
            (i + 1).to_string(),
            storage::csv_cell(&u.handle),
            storage::csv_cell(u.rank.as_deref().unwrap_or("")),
            u.rating.unwrap_or(0).to_string(),
            u.max_rating.unwrap_or(0).to_string(),
            u.contribution.to_string(),
            storage::csv_cell(u.organization.as_deref().unwrap_or("")),
            storage::csv_cell(u.country.as_deref().unwrap_or("")),
        ]);
    }
    storage::csv(rows)
}

#[component]
fn RatedList() -> impl IntoView {
    let data = RwSignal::new(Vec::<api::User>::new());
    let loading = RwSignal::new(false);
    let error = RwSignal::new(String::new());
    let loaded = RwSignal::new(false);

    let active_only = RwSignal::new(true);
    let search = RwSignal::new(String::new());
    let country_filter = RwSignal::new(String::new());
    let org_filter = RwSignal::new(String::new());
    let sort_by = RwSignal::new(String::from("rating"));
    let page = RwSignal::new(1usize);

    let fetch = move || {
        loading.set(true);
        error.set(String::new());
        page.set(1);
        let active = active_only.get_untracked();
        spawn_local(async move {
            match api::user_rated_list(active).await {
                Ok(u) => data.set(u),
                Err(e) => error.set(e),
            }
            loading.set(false);
            loaded.set(true);
        });
    };

    Effect::new(move |_| {
        search.track();
        country_filter.track();
        org_filter.track();
        sort_by.track();
        active_only.track();
        page.set(1);
    });

    let filtered = Memo::new(move |_| {
        let needle = search.get().to_lowercase();
        let country = country_filter.get().to_lowercase();
        let org = org_filter.get().to_lowercase();

        let mut list: Vec<api::User> = data
            .get()
            .into_iter()
            .filter(|u| u.handle.to_lowercase().contains(&needle))
            .filter(|u| {
                country.is_empty()
                    || u.country
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&country)
            })
            .filter(|u| {
                org.is_empty()
                    || u.organization
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&org)
            })
            .collect();

        match sort_by.get().as_str() {
            "max_rating" => list.sort_by_key(|u| -u.max_rating.unwrap_or(-1)),
            "contribution" => list.sort_by_key(|u| -u.contribution),
            "handle" => list.sort_by_key(|a| a.handle.to_lowercase()),
            _ => list.sort_by_key(|u| -u.rating.unwrap_or(-1)),
        }
        list
    });

    let export_csv = Memo::new(move |_| rated_list_csv(&filtered.get()));

    let total = Memo::new(move |_| filtered.with(|f| f.len()));
    let page_count = Memo::new(move |_| (total.get() + PAGE_SIZE - 1).max(1) / PAGE_SIZE);
    let paged = Memo::new(move |_| {
        filtered
            .get()
            .into_iter()
            .skip((page.get().saturating_sub(1)) * PAGE_SIZE)
            .take(PAGE_SIZE)
            .collect::<Vec<_>>()
    });

    view! {
        <Flex gap=FlexGap::Small align=FlexAlign::End style="flex-wrap:wrap;">
            <Field label="Handle contains">
                <Input placeholder="search" value=search input_style="width:220px;"/>
            </Field>
            <Field label="Country contains">
                <Input placeholder="e.g. India" value=country_filter input_style="width:160px;"/>
            </Field>
            <Field label="Organization contains">
                <Input placeholder="e.g. MIT" value=org_filter input_style="width:180px;"/>
            </Field>
            <Field label="Sort">
                <Select default_value="rating" value=sort_by>
                    <option value="rating">"By rating"</option>
                    <option value="max_rating">"By max rating"</option>
                    <option value="contribution">"By contribution"</option>
                    <option value="handle">"By handle A-Z"</option>
                </Select>
            </Field>
            <Checkbox checked=active_only label="Active only"/>
            <Button appearance=ButtonAppearance::Primary on:click=move |_| fetch()>
                "Load rated list"
            </Button>
        </Flex>
        {move || -> AnyView {
            if loading.get() {
                view! { <Loading label="Loading rated list (large download, please wait)".into()/> }.into_any()
            } else if !error.get().is_empty() {
                view! { <ErrorBar message=error/> }.into_any()
            } else if !loaded.get() {
                view! { <Empty text="Press the button to load the global rated list.".into()/> }.into_any()
            } else if total.with(|t| *t == 0) {
                view! { <Empty text="No users match your search.".into()/> }.into_any()
            } else {
                view! {
                    <>
                        <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center style="flex-wrap:wrap;gap:6px;">
                            <Caption1>{format!("{} users matched", thousands(total.with(|t| *t as i64)))}</Caption1>
                            {move || {
                                let content = export_csv.get();
                                view! {
                                    <DownloadButton filename="rated-list.csv" content=content>
                                        "Export CSV"
                                    </DownloadButton>
                                }
                            }}
                        </Flex>
                        <div style="overflow-x:auto;border:1px solid rgba(128,128,128,0.25);border-radius:8px;padding:4px;">
                            <Table>
                                <TableHeader>
                                    <TableRow>
                                        <TableHeaderCell>"#"</TableHeaderCell>
                                        <TableHeaderCell>"Handle"</TableHeaderCell>
                                        <TableHeaderCell>"Rank"</TableHeaderCell>
                                        <TableHeaderCell>"Rating"</TableHeaderCell>
                                        <TableHeaderCell>"Max rating"</TableHeaderCell>
                                        <TableHeaderCell>"Contribution"</TableHeaderCell>
                                        <TableHeaderCell>"Organization"</TableHeaderCell>
                                    </TableRow>
                                </TableHeader>
                                <TableBody>
                                    {move || paged.get().into_iter().enumerate().map(|(i, u)| {
                                        let n = (page.get() - 1) * PAGE_SIZE + i + 1;
                                        let medal = if sort_by.get() == "rating" && page.get() == 1 && i < 3 {
                                            ["\u{1F947}", "\u{1F948}", "\u{1F949}"][i]
                                        } else {
                                            ""
                                        };
                                        let handle = u.handle.clone();
                                        let profile = format!("https://codeforces.com/profile/{handle}");
                                        let rank = u.rank.unwrap_or_default();
                                        let rc = rank_color(&rank);
                                        let rating = u.rating.unwrap_or(0);
                                        let max_rating = u.max_rating.unwrap_or(0);
                                        let mrc = rating_color(max_rating);
                                        let contribution = u.contribution;
                                        let org = truncate(&u.organization.unwrap_or_default(), 40);
                                        view! {
                                            <TableRow>
                                                <TableCell>{medal}{n}</TableCell>
                                                <TableCell>
                                                    <a href=profile target="_blank" rel="noopener" style=format!("color:{rc};font-weight:600;text-decoration:none;")>
                                                        {handle}
                                                    </a>
                                                </TableCell>
                                                <TableCell><span style=format!("color:{rc};")>{rank}</span></TableCell>
                                                <TableCell><b style=format!("color:{rc};")>{rating}</b></TableCell>
                                                <TableCell><b style=format!("color:{mrc};")>{max_rating}</b></TableCell>
                                                <TableCell>{signed_delta(contribution)}</TableCell>
                                                <TableCell><Caption1>{org}</Caption1></TableCell>
                                            </TableRow>
                                        }
                                    }).collect_view()}
                                </TableBody>
                            </Table>
                        </div>
                        <Flex justify=FlexJustify::Center>
                            <Pagination page_count=Signal::from(page_count) page=page/>
                        </Flex>
                    </>
                }
                .into_any()
            }
        }}
    }
}
