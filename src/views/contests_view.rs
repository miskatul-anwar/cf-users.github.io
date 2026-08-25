//! Contests tab: browse all contests (with upcoming pins, type/sort filters
//! and quick jump), inspect standings (medal-tinted, CSV export), rating
//! changes (CSV export), hacks and submissions for any contest.

use crate::api;
use crate::components::*;
use crate::storage::{csv, csv_cell};
use crate::util::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

#[derive(Clone, Copy, PartialEq)]
enum PhaseFilter {
    All,
    Upcoming,
    Running,
    Finished,
}

impl PhaseFilter {
    fn matches(self, phase: &str) -> bool {
        match self {
            Self::All => true,
            Self::Upcoming => phase == "BEFORE",
            Self::Running => phase == "CODING" || phase == "PENDING_SYSTEM_TEST",
            Self::Finished => phase == "FINISHED" || phase == "SYSTEM_TEST" || phase == "CRASHED",
        }
    }
}

fn phase_badge(phase: &str) -> (&'static str, &'static str) {
    match phase {
        "BEFORE" => ("Upcoming", "#0078d4"),
        "CODING" => ("Running", "#008000"),
        "PENDING_SYSTEM_TEST" | "SYSTEM_TEST" => ("System test", "#aa00aa"),
        _ => ("Finished", "#888"),
    }
}

/// Row tint for medal positions (theme-safe translucent colors).
fn medal_tint(pos: i32) -> &'static str {
    match pos {
        1 => "rgba(255,215,0,0.14)",
        2 => "rgba(192,192,192,0.14)",
        3 => "rgba(205,127,50,0.14)",
        _ => "",
    }
}

/// Solid medal edge color for the participant cell border.
fn medal_edge(pos: i32) -> &'static str {
    match pos {
        1 => "#ffd700",
        2 => "#c0c0c0",
        3 => "#cd7f32",
        _ => "",
    }
}

#[component]
pub fn ContestsView() -> impl IntoView {
    let contests = RwSignal::new(Vec::<api::Contest>::new());
    let loading = RwSignal::new(true);
    let error = RwSignal::new(String::new());
    let name_filter = RwSignal::new(String::new());
    let phase_filter = RwSignal::new(String::from("all"));
    let type_filter = RwSignal::new(String::from("all"));
    let sort_filter = RwSignal::new(String::from("new"));
    let jump_input = RwSignal::new(String::new());
    let jump_error = RwSignal::new(String::new());
    let page = RwSignal::new(1usize);
    const PAGE_SIZE: usize = 25;

    // Load the contest list once on mount (cached, already sorted by start).
    spawn_local(async move {
        match api::contest_list_cached().await {
            Ok(c) => contests.set(c),
            Err(e) => error.set(e),
        }
        loading.set(false);
    });

    let filtered = Memo::new(move |_| {
        let needle = name_filter.get().to_lowercase();
        let phase = match phase_filter.get().as_str() {
            "upcoming" => PhaseFilter::Upcoming,
            "running" => PhaseFilter::Running,
            "finished" => PhaseFilter::Finished,
            _ => PhaseFilter::All,
        };
        let tchoice = type_filter.get();
        let mut list: Vec<api::Contest> = contests
            .get()
            .into_iter()
            .filter(|c| phase.matches(&c.phase))
            .filter(|c| c.name.to_lowercase().contains(&needle))
            .filter(|c| tchoice == "all" || contest_div(&c.name) == tchoice.as_str())
            .collect();
        match sort_filter.get().as_str() {
            "old" => list.sort_by_key(|c| c.start_time_seconds.unwrap_or(i64::MAX)),
            "long" => list.sort_by_key(|c| std::cmp::Reverse(c.duration_seconds)),
            _ => list.sort_by_key(|c| std::cmp::Reverse(c.start_time_seconds.unwrap_or(0))),
        }
        list
    });

    let page_count = Memo::new(move |_| (filtered.get().len() + PAGE_SIZE - 1).max(1) / PAGE_SIZE);

    // Any filter change resets pagination.
    Effect::new(move |_| {
        name_filter.track();
        phase_filter.track();
        type_filter.track();
        sort_filter.track();
        page.set(1);
    });
    let paged = Memo::new(move |_| {
        filtered
            .get()
            .into_iter()
            .skip((page.get().saturating_sub(1)) * PAGE_SIZE)
            .take(PAGE_SIZE)
            .collect::<Vec<_>>()
    });

    // Selected contest detail -------------------------------------------------
    let selected: RwSignal<Option<api::Contest>> = RwSignal::new(None);

    // Quick jump to an arbitrary contest ID.
    let open_jump = move |_| {
        let t = jump_input.get_untracked().trim().to_string();
        match t.parse::<i64>() {
            Ok(id) if id > 0 => {
                jump_error.set(String::new());
                jump_input.set(String::new());
                selected.set(Some(api::Contest {
                    id,
                    name: format!("Contest #{id}"),
                    ..Default::default()
                }));
            }
            Ok(_) => jump_error.set("Contest ID must be a positive number.".into()),
            Err(_) => jump_error.set(format!("\u{201c}{t}\u{201d} is not a valid contest ID.")),
        }
    };

    view! {
        <Flex vertical=true gap=FlexGap::Medium>
            <SectionHeader title="All contests".into()/>
            <Flex gap=FlexGap::Small align=FlexAlign::Center style="flex-wrap:wrap;">
                <Input
                    placeholder="Filter by name"
                    value=name_filter
                    input_style="min-width:240px;"
                />
                <Select default_value="all" value=phase_filter>
                    <option value="all">"All phases"</option>
                    <option value="upcoming">"Upcoming"</option>
                    <option value="running">"Running"</option>
                    <option value="finished">"Finished"</option>
                </Select>
                <Select default_value="all" value=type_filter>
                    <option value="all">"All types"</option>
                    <option value="Div. 1">"Div. 1"</option>
                    <option value="Div. 2">"Div. 2"</option>
                    <option value="Div. 3">"Div. 3"</option>
                    <option value="Div. 4">"Div. 4"</option>
                    <option value="Educational">"Educational"</option>
                    <option value="Global">"Global"</option>
                    <option value="ICPC">"ICPC"</option>
                    <option value="Lunchtime">"Lunchtime"</option>
                    <option value="Beginner">"Beginner"</option>
                    <option value="Kotlin">"Kotlin"</option>
                    <option value="Marathon">"Marathon"</option>
                    <option value="Other">"Other"</option>
                </Select>
                <Select default_value="new" value=sort_filter>
                    <option value="new">"Newest first"</option>
                    <option value="old">"Oldest first"</option>
                    <option value="long">"Longest"</option>
                </Select>
                <Input
                    placeholder="Jump to contest ID"
                    value=jump_input
                    input_style="width:170px;"
                />
                <Button
                    appearance=ButtonAppearance::Primary
                    size=ButtonSize::Small
                    on:click=open_jump
                >
                    "Open"
                </Button>
            </Flex>
            {move || {
                let e = jump_error.get();
                (!e.is_empty()).then(|| {
                    view! { <Caption1 style="color:#ff0000;font-weight:600;">{e}</Caption1> }
                })
            }}

            // Pinned upcoming contests --------------------------------------
            {move || -> AnyView {
                if loading.get() || !error.get().is_empty() {
                    ().into_any()
                } else {
                    let mut up: Vec<api::Contest> = contests
                        .get()
                        .into_iter()
                        .filter(|c| c.phase == "BEFORE")
                        .collect();
                    up.sort_by_key(|c| c.start_time_seconds.unwrap_or(i64::MAX));
                    up.truncate(12);
                    if up.is_empty() {
                        ().into_any()
                    } else {
                        let cards = up
                            .into_iter()
                            .map(|c| {
                                let link = format!("https://codeforces.com/contest/{}", c.id);
                                let d = contest_div(&c.name);
                                let dur = format_duration(c.duration_seconds);
                                let starts_in = match c.start_time_seconds {
                                    Some(s) => view! {
                                        <Caption1>
                                            "Starts in: "
                                            <Countdown target_secs=s/>
                                        </Caption1>
                                    }
                                        .into_any(),
                                    None => view! { <Caption1>"Start time TBA"</Caption1> }.into_any(),
                                };
                                let url = link.clone();
                                view! {
                                    <div style="flex:1 1 250px;min-width:230px;max-width:330px;">
                                        <Card>
                                            <a href=link target="_blank" rel="noopener">
                                                <b>{truncate(&c.name, 52)}</b>
                                            </a>
                                            <div style="display:flex;gap:8px;align-items:center;margin-top:4px;flex-wrap:wrap;">
                                                <span style="border:1px solid rgba(128,128,128,0.25);background:rgba(128,128,128,0.06);border-radius:999px;padding:0 8px;font-size:0.75rem;font-weight:600;">
                                                    {d}
                                                </span>
                                                <Caption1>{dur}</Caption1>
                                            </div>
                                            <div style="margin-top:4px;">{starts_in}</div>
                                            <div style="margin-top:6px;">
                                                <Button
                                                    appearance=ButtonAppearance::Subtle
                                                    size=ButtonSize::Small
                                                    on:click=move |_| {
                                                        let _ = window().open_with_url_and_target(&url, "_blank");
                                                    }
                                                >
                                                    "Open on Codeforces"
                                                </Button>
                                            </div>
                                        </Card>
                                    </div>
                                }
                            })
                            .collect_view();
                        view! {
                            <div>
                                <Caption1 style="color:#0078d4;font-weight:700;">"Upcoming"</Caption1>
                                <div style="display:flex;flex-wrap:wrap;gap:10px;">{cards}</div>
                            </div>
                        }
                        .into_any()
                    }
                }
            }}

            {move || -> AnyView {
                if loading.get() {
                    view! { <Loading label="Loading contests".into()/> }.into_any()
                } else if !error.get().is_empty() {
                    view! { <ErrorBar message=error/> }.into_any()
                } else if filtered.with(|f| f.is_empty()) {
                    view! { <Empty text="No contests match your filter.".into()/> }.into_any()
                } else {
                    view! {
                        <>
                            <Table>
                                <TableHeader>
                                    <TableRow>
                                        <TableHeaderCell>"ID"</TableHeaderCell>
                                        <TableHeaderCell>"Name"</TableHeaderCell>
                                        <TableHeaderCell>"Phase"</TableHeaderCell>
                                        <TableHeaderCell>"Start (UTC)"</TableHeaderCell>
                                        <TableHeaderCell>"Duration"</TableHeaderCell>
                                        <TableHeaderCell>"Details"</TableHeaderCell>
                                    </TableRow>
                                </TableHeader>
                                <TableBody>
                                    {move || paged.get().into_iter().map(|c| {
                                        let id = c.id;
                                        let link = format!("https://codeforces.com/contest/{id}");
                                        let start = c
                                            .start_time_seconds
                                            .map(format_time)
                                            .unwrap_or_else(|| "\u{2014}".into());
                                        let dur = format_duration(c.duration_seconds);
                                        let (badge, color) = phase_badge(&c.phase);
                                        let frozen = c.frozen.then(|| view!{ <Caption1>" \u{2744}"</Caption1> });
                                        let cname = truncate(&c.name, 60);
                                        let c_for_btn = c.clone();
                                        view! {
                                            <TableRow>
                                                <TableCell>{id}</TableCell>
                                                <TableCell><a href=link target="_blank" rel="noopener">{cname}</a></TableCell>
                                                <TableCell>
                                                    <span style=format!("color:{color};font-weight:600;")>{badge}</span>
                                                    {frozen}
                                                </TableCell>
                                                <TableCell>{start}</TableCell>
                                                <TableCell>{dur}</TableCell>
                                                <TableCell>
                                                    <Button
                                                        appearance=ButtonAppearance::Subtle
                                                        size=ButtonSize::Small
                                                        on:click=move |_| selected.set(Some(c_for_btn.clone()))
                                                    >
                                                        "Inspect"
                                                    </Button>
                                                </TableCell>
                                            </TableRow>
                                        }
                                    }).collect_view()}
                                </TableBody>
                            </Table>
                            <Flex justify=FlexJustify::Center>
                                <Pagination page_count=Signal::from(page_count) page=page/>
                            </Flex>
                        </>
                    }
                    .into_any()
                }
            }}

            {move || selected.get().map(|contest| view! { <ContestDetail contest/> })}
        </Flex>
    }
}

#[component]
fn ContestDetail(contest: api::Contest) -> impl IntoView {
    let id = contest.id;
    let tab = RwSignal::new(String::from("standings"));
    let url = format!("https://codeforces.com/contest/{id}");
    let (badge, color) = phase_badge(&contest.phase);
    let frozen_chip = contest
        .frozen
        .then(|| view! { <Caption1>" \u{2744}"</Caption1> });
    let meta = format!(
        "Starts {} \u{00b7} Duration {}",
        contest
            .start_time_seconds
            .map(format_time)
            .unwrap_or_else(|| "\u{2014}".into()),
        format_duration(contest.duration_seconds),
    );

    view! {
        <div style="margin-top:8px;">
        <Card>
            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center style="flex-wrap:wrap;" gap=FlexGap::Small>
                <div>
                    <Text style="font-size:1.1em;font-weight:700;">
                        {format!("{} \u{2014} {}", contest.id, contest.name.clone())}
                    </Text>
                    <div style="display:flex;align-items:center;gap:8px;margin-top:2px;flex-wrap:wrap;">
                        <span style=format!("color:{color};font-weight:600;font-size:0.85rem;")>{badge}</span>
                        {frozen_chip}
                        <Caption1>{meta}</Caption1>
                    </div>
                </div>
                <CopyLinkButton text=url/>
            </Flex>
            <Divider/>
            <TabList selected_value=tab>
                <Tab value="standings">"Standings"</Tab>
                <Tab value="rating">"Rating changes"</Tab>
                <Tab value="hacks">"Hacks"</Tab>
                <Tab value="status">"Submissions"</Tab>
            </TabList>
            {move || match tab.get().as_str() {
                "rating" => view! { <RatingChangesPanel contest_id=id/> }.into_any(),
                "hacks" => view! { <HacksPanel contest_id=id/> }.into_any(),
                "status" => view! { <ContestStatusPanel contest_id=id/> }.into_any(),
                _ => view! { <StandingsPanel contest_id=id/> }.into_any(),
            }}
        </Card>
        </div>
    }
}

// ---------------------------------------------------------------------------

#[component]
fn StandingsPanel(contest_id: i64) -> impl IntoView {
    let from_input = RwSignal::new(String::from("1"));
    let count_input = RwSignal::new(String::from("50"));
    let handles_input = RwSignal::new(String::new());
    let show_unofficial = RwSignal::new(false);

    let data = RwSignal::new(None::<api::ContestStandingsResult>);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(String::new());

    let fetch = move || {
        let cid = contest_id;
        let from: u32 = from_input.get_untracked().parse().unwrap_or(1);
        let count: u32 = count_input.get_untracked().parse().unwrap_or(50).min(10000);
        let handles: Vec<String> = handles_input
            .get_untracked()
            .split([',', ';'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        let unofficial = show_unofficial.get_untracked();
        loading.set(true);
        error.set(String::new());
        spawn_local(async move {
            match api::contest_standings(
                cid,
                from,
                count,
                &handles.iter().map(String::as_str).collect::<Vec<_>>(),
                unofficial,
            )
            .await
            {
                Ok(r) => data.set(Some(r)),
                Err(e) => error.set(e),
            }
            loading.set(false);
        });
    };

    // Initial load.
    Effect::new(move |_| {
        fetch();
    });

    view! {
        <Flex vertical=true gap=FlexGap::Small>
            <Flex gap=FlexGap::Small align=FlexAlign::End style="flex-wrap:wrap;">
                <Field label="From rank">
                    <Input value=from_input input_style="width:90px;"/>
                </Field>
                <Field label="Count">
                    <Input value=count_input input_style="width:90px;"/>
                </Field>
                <Field label="Handles (; separated)">
                    <Input placeholder="optional" value=handles_input input_style="width:220px;"/>
                </Field>
                <Checkbox checked=show_unofficial label="Show unofficial"/>
                <Button appearance=ButtonAppearance::Primary on:click=move |_| fetch()>
                    "Load"
                </Button>
            </Flex>
            {move || -> AnyView {
                if loading.get() {
                    view! { <Loading label="Loading standings".into()/> }.into_any()
                } else if !error.get().is_empty() {
                    view! { <ErrorBar message=error/> }.into_any()
                } else if data.with(|d| d.is_none()) {
                    view! { <Empty text="No data.".into()/> }.into_any()
                } else {
                    let res = data.get_untracked().unwrap();
                    let problems = res.problems.clone();
                    let rows = res.rows.clone();

                    // CSV export: fixed header + one column per problem code.
                    let mut header_row: Vec<String> =
                        ["Pos", "Participant", "Team", "Points", "Penalty", "Hacks+"]
                            .iter()
                            .map(|s| s.to_string())
                            .collect();
                    header_row.extend(problems.iter().map(|p| p.code()));
                    let mut csv_rows = vec![header_row];
                    for row in &rows {
                        let mut r = vec![
                            row.position.to_string(),
                            csv_cell(&row.party.handles()),
                            csv_cell(row.party.team_name.as_deref().unwrap_or("")),
                            format!("{}", row.points),
                            row.penalty.to_string(),
                            row.successful_hack_count.to_string(),
                        ];
                        for pr in &row.problem_results {
                            r.push(if pr.problem_result_type == "FINAL" {
                                format!("{:.0}", pr.points)
                            } else if pr.rejected_attempt_count > 0 {
                                format!("-{}", pr.rejected_attempt_count)
                            } else {
                                String::new()
                            });
                        }
                        csv_rows.push(r);
                    }
                    let csv_content = csv(csv_rows);
                    let csv_name = format!("standings_{contest_id}.csv");

                    view! {
                        <Flex gap=FlexGap::Small align=FlexAlign::Center style="flex-wrap:wrap;">
                            <Caption1>{format!("Showing {} rows", res.rows.len())}</Caption1>
                            <DownloadButton filename=csv_name content=csv_content>"Export CSV"</DownloadButton>
                        </Flex>
                        <div style="overflow-x:auto;border:1px solid rgba(128,128,128,0.25);border-radius:8px;padding:4px;">
                            <Table>
                                <TableHeader>
                                    <TableRow>
                                        <TableHeaderCell>"#"</TableHeaderCell>
                                        <TableHeaderCell>"Participant"</TableHeaderCell>
                                        <TableHeaderCell>"Points"</TableHeaderCell>
                                        <TableHeaderCell>"Penalty"</TableHeaderCell>
                                        <TableHeaderCell>"Hacks"</TableHeaderCell>
                                        {problems
                                            .iter()
                                            .map(|p| {
                                                let code = p.code();
                                                view! {
                                                    <TableHeaderCell min_width=60.0>{code}</TableHeaderCell>
                                                }
                                            })
                                            .collect_view()}
                                    </TableRow>
                                </TableHeader>
                                <TableBody>
                                    {rows.into_iter().map(|row| {
                                        let pos = row.position;
                                        let tint = medal_tint(pos);
                                        let edge = medal_edge(pos);
                                        let cell_style = if tint.is_empty() {
                                            String::new()
                                        } else {
                                            format!(
                                                "background:{tint};border-radius:4px;padding:2px 6px;margin:-2px -6px;display:inline-block;",
                                            )
                                        };
                                        let part_style = if edge.is_empty() {
                                            cell_style.clone()
                                        } else {
                                            format!("{cell_style}border-left:3px solid {edge};")
                                        };
                                        let team = row.party.team_name.clone();
                                        let handle_links = row
                                            .party
                                            .handles()
                                            .split(", ")
                                            .map(str::to_string)
                                            .enumerate()
                                            .map(|(i, h)| {
                                                let sep: &'static str = if i == 0 { "" } else { ", " };
                                                view! {
                                                    <span>{sep}<HandleLink handle=h/></span>
                                                }
                                            })
                                            .collect_view();
                                        let who_view = match team {
                                            Some(t) => view! {
                                                <div>
                                                    <b>{t}</b>
                                                    <br/>
                                                    <Caption1>{handle_links}</Caption1>
                                                </div>
                                            }
                                                .into_any(),
                                            None => view! { <div>{handle_links}</div> }.into_any(),
                                        };
                                        let pts = row.points;
                                        let pen = row.penalty;
                                        let hacks = format!(
                                            "+{} / -{}",
                                            row.successful_hack_count,
                                            row.unsuccessful_hack_count
                                        );
                                        let cells = row.problem_results.iter().map(|pr| {
                                            let score = pr.points;
                                            let solved = pr.problem_result_type == "FINAL";
                                            let attempts = pr.rejected_attempt_count;
                                            let cell_color = if solved { "#008000" } else if attempts > 0 { "#ff0000" } else { "#999" };
                                            let st = cell_style.clone();
                                            view! {
                                                <TableCell>
                                                    <span style=st>
                                                        <span style=format!("color:{cell_color};")>
                                                            {if solved { format!("{score:.0}") } else if attempts > 0 { format!("-{attempts}") } else { "\u{2014}".to_string() }}
                                                        </span>
                                                    </span>
                                                </TableCell>
                                            }
                                        }).collect_view();
                                        let st_pos = cell_style.clone();
                                        let st_pts = cell_style.clone();
                                        let st_pen = cell_style.clone();
                                        view! {
                                            <TableRow>
                                                <TableCell><span style=st_pos>{pos}</span></TableCell>
                                                <TableCell><span style=part_style>{who_view}</span></TableCell>
                                                <TableCell><span style=st_pts>{format!("{pts:.1}")}</span></TableCell>
                                                <TableCell><span style=st_pen>{pen}</span></TableCell>
                                                <TableCell><span style=cell_style>{hacks}</span></TableCell>
                                                {cells}
                                            </TableRow>
                                        }
                                    }).collect_view()}
                                </TableBody>
                            </Table>
                        </div>
                    }
                    .into_any()
                }
            }}
        </Flex>
    }
}

#[component]
fn RatingChangesPanel(contest_id: i64) -> impl IntoView {
    let data = RwSignal::new(Vec::<api::RatingChange>::new());
    let loading = RwSignal::new(true);
    let error = RwSignal::new(String::new());

    Effect::new(move |_| {
        let cid = contest_id;
        spawn_local(async move {
            match api::contest_rating_changes(cid).await {
                Ok(c) => data.set(c),
                Err(e) => error.set(e),
            }
            loading.set(false);
        });
    });

    let stats = move || {
        let d = data.get_untracked();
        (!d.is_empty()).then(|| {
            let deltas: Vec<i32> = d.iter().map(|c| c.new_rating - c.old_rating).collect();
            let best_up = deltas.iter().max().copied().unwrap_or(0);
            let worst_down = deltas.iter().min().copied().unwrap_or(0);

            // CSV export: Handle,Date,Contest,Rank,Old,New,Delta.
            let csv_rows: Vec<Vec<String>> = std::iter::once(
                ["Handle", "Date", "Contest", "Rank", "Old", "New", "Delta"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            )
            .chain(d.iter().map(|c| {
                vec![
                    csv_cell(&c.handle),
                    format_date(c.rating_update_time_seconds),
                    csv_cell(&c.contest_name),
                    c.rank.to_string(),
                    c.old_rating.to_string(),
                    c.new_rating.to_string(),
                    signed_delta(c.new_rating - c.old_rating),
                ]
            }))
            .collect();
            let csv_content = csv(csv_rows);
            let fname = format!("{contest_id}_rating_changes.csv");

            view! {
                <Flex gap=FlexGap::Large align=FlexAlign::Center style="flex-wrap:wrap;">
                    <Caption1>{format!("Rated participants: {}", thousands(d.len() as i64))}</Caption1>
                    <Caption1 style="color:#008000;font-weight:600;">{format!("Best delta: {}", signed_delta(best_up))}</Caption1>
                    <Caption1 style="color:#ff0000;font-weight:600;">{format!("Worst delta: {}", signed_delta(worst_down))}</Caption1>
                    <DownloadButton filename=fname content=csv_content>"Export CSV"</DownloadButton>
                </Flex>
            }
        })
    };

    view! {
        <Flex vertical=true gap=FlexGap::Small>
            {stats}
            {move || -> AnyView {
                if loading.get() {
                    view! { <Loading label="Loading rating changes".into()/> }.into_any()
                } else if !error.get().is_empty() {
                    view! { <ErrorBar message=error/> }.into_any()
                } else if data.with(|d| d.is_empty()) {
                    view! { <Empty text="This contest is unrated or has no published rating changes.".into()/> }.into_any()
                } else {
                    view! {
                        <div style="max-height:480px;overflow:auto;border:1px solid rgba(128,128,128,0.25);border-radius:8px;padding:4px;">
                            <RatingChangeTable changes=data.get_untracked() show_handle=true/>
                        </div>
                    }
                    .into_any()
                }
            }}
        </Flex>
    }
}

#[component]
fn HacksPanel(contest_id: i64) -> impl IntoView {
    let data = RwSignal::new(Vec::<api::Hack>::new());
    let loading = RwSignal::new(true);
    let error = RwSignal::new(String::new());

    Effect::new(move |_| {
        let cid = contest_id;
        spawn_local(async move {
            match api::contest_hacks(cid).await {
                Ok(h) => data.set(h),
                Err(e) => error.set(e),
            }
            loading.set(false);
        });
    });

    view! {
        <Flex vertical=true gap=FlexGap::Small>
            {move || -> AnyView {
                if loading.get() {
                    view! { <Loading label="Loading hacks".into()/> }.into_any()
                } else if !error.get().is_empty() {
                    view! { <ErrorBar message=error/> }.into_any()
                } else if data.with(|d| d.is_empty()) {
                    view! { <Empty text="No hacks recorded for this contest.".into()/> }.into_any()
                } else {
                    let rows = data.get_untracked().into_iter().enumerate().map(|(i, h)| {
                        let n = i + 1;
                        let hacker = h.hacker.handles();
                        let defender = h.defender.handles();
                        let when = format_time(h.creation_time_seconds);
                        let verdict = h.verdict.unwrap_or_else(|| "PENDING".into());
                        let vc = if verdict == "SUCCESSFUL" { "#008000" } else { "#ff0000" };
                        view! {
                            <TableRow>
                                <TableCell>{n}</TableCell>
                                <TableCell>{when}</TableCell>
                                <TableCell>{hacker}</TableCell>
                                <TableCell>{defender}</TableCell>
                                <TableCell><b style=format!("color:{vc};")>{verdict.replace('_', " ")}</b></TableCell>
                            </TableRow>
                        }
                    }).collect_view();
                    view! {
                        <div style="max-height:480px;overflow:auto;border:1px solid rgba(128,128,128,0.25);border-radius:8px;padding:4px;">
                            <Table>
                                <TableHeader>
                                    <TableRow>
                                        <TableHeaderCell>"#"</TableHeaderCell>
                                        <TableHeaderCell>"Time"</TableHeaderCell>
                                        <TableHeaderCell>"Hacker"</TableHeaderCell>
                                        <TableHeaderCell>"Defender"</TableHeaderCell>
                                        <TableHeaderCell>"Verdict"</TableHeaderCell>
                                    </TableRow>
                                </TableHeader>
                                <TableBody>{rows}</TableBody>
                            </Table>
                        </div>
                    }
                    .into_any()
                }
            }}
        </Flex>
    }
}

#[component]
fn ContestStatusPanel(contest_id: i64) -> impl IntoView {
    let handle_input = RwSignal::new(String::new());
    let count_input = RwSignal::new(String::from("50"));
    let data = RwSignal::new(Vec::<api::Submission>::new());
    let loading = RwSignal::new(false);
    let error = RwSignal::new(String::new());

    let fetch = move || {
        let cid = contest_id;
        let count: u32 = count_input.get_untracked().parse().unwrap_or(50).min(1000);
        let handle = handle_input.get_untracked().trim().to_string();
        loading.set(true);
        error.set(String::new());
        spawn_local(async move {
            match api::contest_status(cid, 1, count, Some(&handle)).await {
                Ok(s) => data.set(s),
                Err(e) => error.set(e),
            }
            loading.set(false);
        });
    };

    Effect::new(move |_| {
        fetch();
    });

    view! {
        <Flex vertical=true gap=FlexGap::Small>
            <Flex gap=FlexGap::Small align=FlexAlign::End style="flex-wrap:wrap;">
                <Field label="Handle filter">
                    <Input placeholder="optional" value=handle_input input_style="width:200px;"/>
                </Field>
                <Field label="Count">
                    <Input value=count_input input_style="width:90px;"/>
                </Field>
                <Button appearance=ButtonAppearance::Primary on:click=move |_| fetch()>
                    "Load"
                </Button>
            </Flex>
            {move || -> AnyView {
                if loading.get() {
                    view! { <Loading label="Loading submissions".into()/> }.into_any()
                } else if !error.get().is_empty() {
                    view! { <ErrorBar message=error/> }.into_any()
                } else if data.with(|d| d.is_empty()) {
                    view! { <Empty text="No submissions found.".into()/> }.into_any()
                } else {
                    view! { <SubmissionTable subs=data.get_untracked()/> }.into_any()
                }
            }}
        </Flex>
    }
}
