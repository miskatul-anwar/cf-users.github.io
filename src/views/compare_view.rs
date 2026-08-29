//! Compare tab: side-by-side comparison of two or three Codeforces users.

use crate::api;
use crate::components::*;
use crate::storage;
use crate::util::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashSet;
use thaw::*;

const PALETTE: [&str; 3] = ["#0078d4", "#aa00aa", "#008000"];
const WIN_COLOR: &str = "#008000";

#[derive(Clone, Copy)]
struct SolveStat {
    solved: i64,
    ok: i64,
    subs: i64,
}

struct Part {
    slot: usize,
    handle: String,
    user: api::User,
    changes: Vec<api::RatingChange>,
    solve: Option<SolveStat>,
}

struct CellSpec {
    text: String,
    color: Option<&'static str>,
    win: bool,
}

/// Which cells tie for the best (max or min) value in a row.
fn winners(vals: &[f64], max_wins: bool) -> Vec<bool> {
    let best = if max_wins {
        vals.iter().copied().fold(f64::NEG_INFINITY, f64::max)
    } else {
        vals.iter().copied().fold(f64::INFINITY, f64::min)
    };
    vals.iter().map(|v| *v == best).collect()
}

fn signed_f64(v: f64) -> String {
    if v >= 0.0 {
        format!("+{v:.1}")
    } else {
        format!("{v:.1}")
    }
}

fn metric_row(label: &'static str, cells: Vec<CellSpec>) -> impl IntoView {
    let tds = cells
        .into_iter()
        .map(|c| match (c.win, c.color) {
            (true, _) => {
                view! { <TableCell><b style=format!("color:{WIN_COLOR};")>{c.text}</b></TableCell> }
                    .into_any()
            }
            (false, Some(col)) => {
                view! { <TableCell><span style=format!("color:{col};")>{c.text}</span></TableCell> }
                    .into_any()
            }
            (false, None) => view! { <TableCell>{c.text}</TableCell> }.into_any(),
        })
        .collect_view();
    view! {
        <TableRow>
            <TableCell><b>{label}</b></TableCell>
            {tds}
        </TableRow>
    }
}

#[component]
pub fn CompareView(initial_a: Option<String>, initial_b: Option<String>) -> impl IntoView {
    // Inputs -----------------------------------------------------------------
    let a_input = RwSignal::new(String::new());
    let b_input = RwSignal::new(String::new());
    let c_input = RwSignal::new(String::new());
    let third_enabled = RwSignal::new(false);
    let load_solves = RwSignal::new(false);

    // Data: three fixed slots -------------------------------------------------
    let infos = RwSignal::new(std::array::from_fn(|_| None::<api::User>));
    let changess = RwSignal::new([Vec::<api::RatingChange>::new(), Vec::new(), Vec::new()]);
    let solves = RwSignal::new([None::<SolveStat>; 3]);
    let loading = RwSignal::new([false; 3]);
    let solves_loading = RwSignal::new([false; 3]);
    let errs = RwSignal::new([String::new(), String::new(), String::new()]);
    let pending = RwSignal::new([0u32; 3]);
    let solves_req = RwSignal::new(false);
    let ran = RwSignal::new(false);
    let ctrl_err = RwSignal::new(String::new());

    let finish = move |i: usize| {
        pending.update(|p| {
            p[i] = p[i].saturating_sub(1);
            if p[i] == 0 {
                loading.update(|l| l[i] = false);
            }
        });
    };

    let compare = move || {
        let want_solves = load_solves.get_untracked();
        let hs: [Option<String>; 3] = [
            parse_handles(&a_input.get_untracked()).into_iter().next(),
            parse_handles(&b_input.get_untracked()).into_iter().next(),
            third_enabled
                .get_untracked()
                .then(|| parse_handles(&c_input.get_untracked()).into_iter().next())
                .flatten(),
        ];
        let active: Vec<usize> = (0..3).filter(|i| hs[*i].is_some()).collect();
        if active.len() < 2 {
            ctrl_err.set("Enter handles in at least two slots to compare.".into());
            return;
        }
        ctrl_err.set(String::new());
        ran.set(true);
        infos.set([None, None, None]);
        changess.set(Default::default());
        solves.set([None, None, None]);
        errs.set(Default::default());
        let mut pend = [0u32; 3];
        let mut ld = [false; 3];
        for &i in &active {
            pend[i] = if want_solves { 3 } else { 2 };
            ld[i] = true;
        }
        pending.set(pend);
        loading.set(ld);
        solves_req.set(want_solves);

        for i in active {
            let h = hs[i].clone().unwrap();
            let h2 = h.clone();
            let h3 = h.clone();
            spawn_local(async move {
                let res = api::user_info(&[h.as_str()]).await;
                // Bail out if the view was unmounted mid-request.
                if !infos.is_disposed() {
                    match res {
                        Ok(u) => infos.update(|a| a[i] = u.into_iter().next()),
                        Err(e) => errs.update(|a| a[i] = format!("{h}: {e}")),
                    }
                    finish(i);
                }
            });
            spawn_local(async move {
                let res = api::user_rating_cached(&h2).await;
                if !changess.is_disposed() {
                    match res {
                        Ok(c) => changess.update(|a| a[i] = c),
                        Err(e) => errs.update(|a| {
                            if a[i].is_empty() {
                                a[i] = format!("{h2}: {e}");
                            }
                        }),
                    }
                    finish(i);
                }
            });
            if want_solves {
                solves_loading.update(|a| a[i] = true);
                spawn_local(async move {
                    let res = api::user_status_cached(&h3, 2000).await;
                    if !solves.is_disposed() {
                        match res {
                            Ok(subs) => {
                                let ok_count = subs
                                    .iter()
                                    .filter(|s| s.verdict.as_deref() == Some("OK"))
                                    .count();
                                let solved: HashSet<String> = subs
                                    .iter()
                                    .filter(|s| s.verdict.as_deref() == Some("OK"))
                                    .map(|s| problem_key(s.contest_id, &s.problem.index))
                                    .collect();
                                let stat = SolveStat {
                                    solved: solved.len() as i64,
                                    ok: ok_count as i64,
                                    subs: subs.len() as i64,
                                };
                                solves.update(|a| a[i] = Some(stat));
                            }
                            Err(e) => errs.update(|a| {
                                if a[i].is_empty() {
                                    a[i] = format!("{h3}: {e}");
                                }
                            }),
                        }
                        solves_loading.update(|a| a[i] = false);
                        finish(i);
                    }
                });
            }
        }
    };

    // Prefill from props; auto-run when both handles are present.
    let auto_run = initial_a.is_some() && initial_b.is_some();
    if let Some(a) = initial_a {
        a_input.set(a);
    }
    if let Some(b) = initial_b {
        b_input.set(b);
    }
    if auto_run {
        Effect::new(move |_| {
            compare();
        });
    }

    // Shareable deep link -----------------------------------------------------
    let share_link = Memo::new(move |_| {
        let base = window()
            .location()
            .href()
            .unwrap_or_default()
            .split('#')
            .next()
            .unwrap_or_default()
            .to_string();
        let mut hs: Vec<String> = [
            parse_handles(&a_input.get()).into_iter().next(),
            parse_handles(&b_input.get()).into_iter().next(),
        ]
        .into_iter()
        .flatten()
        .collect();
        if third_enabled.get()
            && let Some(h) = parse_handles(&c_input.get()).into_iter().next()
        {
            hs.push(h);
        }
        if hs.is_empty() {
            base
        } else {
            format!("{base}#/compare/{}", hs.join("/"))
        }
    });

    // Participants currently visible ------------------------------------------
    let collect_parts = move || -> Vec<Part> {
        let third = third_enabled.get();
        let inf = infos.get();
        let chg = changess.get();
        let slv = solves.get();
        (0..3)
            .filter(|&i| i != 2 || third)
            .filter_map(|i| {
                let user = inf[i].clone()?;
                Some(Part {
                    slot: i,
                    handle: user.handle.clone(),
                    user,
                    changes: chg[i].clone(),
                    solve: slv[i],
                })
            })
            .collect()
    };

    let csv_content = Memo::new(move |_| {
        let parts = collect_parts();
        if parts.is_empty() {
            return String::new();
        }
        let show_solve = solves_req.get() && parts.iter().all(|p| p.solve.is_some());
        let mut head: Vec<String> = [
            "Handle",
            "Current",
            "Max",
            "Rank",
            "Contests",
            "BestPlace",
            "AvgDelta",
            "Contribution",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        if show_solve {
            head.extend(["Solved", "Acceptance"].iter().map(|s| s.to_string()));
        }
        let mut rows = vec![head];
        for p in &parts {
            let u = &p.user;
            let n = p.changes.len();
            let avg = if n == 0 {
                0.0
            } else {
                p.changes
                    .iter()
                    .map(|c| (c.new_rating - c.old_rating) as f64)
                    .sum::<f64>()
                    / n as f64
            };
            let best_place = p.changes.iter().map(|c| c.rank).min();
            let mut row = vec![
                storage::csv_cell(&p.handle),
                storage::csv_cell(&u.rating.unwrap_or(0).to_string()),
                storage::csv_cell(&u.max_rating.unwrap_or(0).to_string()),
                storage::csv_cell(&u.rank.clone().unwrap_or_else(|| "unrated".into())),
                storage::csv_cell(&n.to_string()),
                storage::csv_cell(
                    &best_place
                        .map(|r| r.to_string())
                        .unwrap_or_else(|| "-".into()),
                ),
                storage::csv_cell(&signed_f64(avg)),
                storage::csv_cell(&signed_delta(u.contribution)),
            ];
            if show_solve {
                let s = p.solve.unwrap_or(SolveStat {
                    solved: 0,
                    ok: 0,
                    subs: 0,
                });
                row.push(storage::csv_cell(&s.solved.to_string()));
                row.push(storage::csv_cell(&format!("{:.1}%", pct(s.ok, s.subs))));
            }
            rows.push(row);
        }
        storage::csv(rows)
    });

    // Views -------------------------------------------------------------------
    let on_enter = move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Enter" {
            compare();
        }
    };

    let caption = move |i: usize| {
        move || -> AnyView {
            if loading.get()[i] {
                view! { <Caption1 style="color:#888;">"Loading\u{2026}"</Caption1> }.into_any()
            } else if solves_loading.get()[i] {
                view! { <Caption1 style="color:#888;">"Counting solves\u{2026}"</Caption1> }
                    .into_any()
            } else if !errs.get()[i].is_empty() {
                let e = errs.get()[i].clone();
                view! { <Caption1 style="color:#c50f1f;">{e}</Caption1> }.into_any()
            } else {
                view! { <Caption1 style="visibility:hidden;">"-"</Caption1> }.into_any()
            }
        }
    };

    let results = move || -> AnyView {
        if !ran.get() {
            return view! {
                <Empty text="Enter two handles to compare their Codeforces journeys.".into()/>
            }
            .into_any();
        }
        if loading.get().iter().any(|&l| l) {
            return view! { <Loading label="Comparing users".into()/> }.into_any();
        }
        let parts = collect_parts();
        if parts.is_empty() {
            return view! {
                <Empty text="No valid profiles loaded \u{2014} check the highlighted handles and try again.".into()/>
            }
            .into_any();
        }

        let series: Vec<SeriesChanges> = parts
            .iter()
            .filter(|p| !p.changes.is_empty())
            .map(|p| SeriesChanges {
                name: p.handle.clone(),
                color: PALETTE[p.slot],
                changes: p.changes.clone(),
            })
            .collect();

        let multi = parts.len() > 1;
        let show_solve = solves_req.get() && parts.iter().all(|p| p.solve.is_some());
        let now = storage::now_secs();

        let mut rows: Vec<(&'static str, Vec<CellSpec>)> = Vec::new();

        {
            let vals: Vec<f64> = parts
                .iter()
                .map(|p| p.user.rating.unwrap_or(0) as f64)
                .collect();
            let w = winners(&vals, true);
            rows.push((
                "Current rating",
                parts
                    .iter()
                    .enumerate()
                    .map(|(k, p)| CellSpec {
                        text: p.user.rating.unwrap_or(0).to_string(),
                        color: Some(rating_color(p.user.rating.unwrap_or(0))),
                        win: multi && w[k],
                    })
                    .collect(),
            ));
        }
        {
            let vals: Vec<f64> = parts
                .iter()
                .map(|p| p.user.max_rating.unwrap_or(0) as f64)
                .collect();
            let w = winners(&vals, true);
            rows.push((
                "Max rating",
                parts
                    .iter()
                    .enumerate()
                    .map(|(k, p)| CellSpec {
                        text: p.user.max_rating.unwrap_or(0).to_string(),
                        color: Some(rating_color(p.user.max_rating.unwrap_or(0))),
                        win: multi && w[k],
                    })
                    .collect(),
            ));
        }
        rows.push((
            "Rank",
            parts
                .iter()
                .map(|p| {
                    let rk = p.user.rank.clone().unwrap_or_else(|| "unrated".into());
                    let rc = rank_color(&rk);
                    CellSpec {
                        text: rk,
                        color: Some(rc),
                        win: false,
                    }
                })
                .collect(),
        ));
        {
            let vals: Vec<f64> = parts.iter().map(|p| p.changes.len() as f64).collect();
            let w = winners(&vals, true);
            rows.push((
                "Contests",
                parts
                    .iter()
                    .enumerate()
                    .map(|(k, p)| CellSpec {
                        text: thousands(p.changes.len() as i64),
                        color: None,
                        win: multi && w[k],
                    })
                    .collect(),
            ));
        }
        {
            let vals: Vec<f64> = parts
                .iter()
                .map(|p| {
                    p.changes
                        .iter()
                        .map(|c| c.rank as f64)
                        .fold(f64::INFINITY, f64::min)
                })
                .collect();
            let w = winners(&vals, false);
            rows.push((
                "Best place",
                parts
                    .iter()
                    .enumerate()
                    .map(|(k, _p)| CellSpec {
                        text: if vals[k].is_infinite() {
                            "\u{2014}".into()
                        } else {
                            (vals[k] as i32).to_string()
                        },
                        color: None,
                        win: multi && w[k] && !vals[k].is_infinite(),
                    })
                    .collect(),
            ));
        }
        {
            let vals: Vec<f64> = parts
                .iter()
                .map(|p| {
                    let n = p.changes.len();
                    if n == 0 {
                        0.0
                    } else {
                        p.changes
                            .iter()
                            .map(|c| (c.new_rating - c.old_rating) as f64)
                            .sum::<f64>()
                            / n as f64
                    }
                })
                .collect();
            let w = winners(&vals, true);
            rows.push((
                "Average delta",
                parts
                    .iter()
                    .enumerate()
                    .map(|(k, _p)| {
                        let col = if vals[k] >= 0.0 { "#008000" } else { "#ff0000" };
                        CellSpec {
                            text: signed_f64(vals[k]),
                            color: Some(col),
                            win: multi && w[k],
                        }
                    })
                    .collect(),
            ));
        }
        {
            let vals: Vec<f64> = parts.iter().map(|p| p.user.contribution as f64).collect();
            let w = winners(&vals, true);
            rows.push((
                "Contribution",
                parts
                    .iter()
                    .enumerate()
                    .map(|(k, p)| {
                        let c = p.user.contribution;
                        let col = if c >= 0 { "#008000" } else { "#ff0000" };
                        CellSpec {
                            text: signed_delta(c),
                            color: Some(col),
                            win: multi && w[k],
                        }
                    })
                    .collect(),
            ));
        }
        rows.push((
            "Registered",
            parts
                .iter()
                .map(|p| CellSpec {
                    text: format_date(p.user.registration_time_seconds),
                    color: None,
                    win: false,
                })
                .collect(),
        ));
        rows.push((
            "Last online",
            parts
                .iter()
                .map(|p| CellSpec {
                    text: rel_time(p.user.last_online_time_seconds, now),
                    color: None,
                    win: false,
                })
                .collect(),
        ));
        {
            let vals: Vec<f64> = parts
                .iter()
                .map(|p| p.user.friend_of_count as f64)
                .collect();
            let w = winners(&vals, true);
            rows.push((
                "Friend of",
                parts
                    .iter()
                    .enumerate()
                    .map(|(k, p)| CellSpec {
                        text: thousands(p.user.friend_of_count as i64),
                        color: None,
                        win: multi && w[k],
                    })
                    .collect(),
            ));
        }
        if show_solve {
            let sv: Vec<f64> = parts
                .iter()
                .map(|p| p.solve.map(|s| s.solved as f64).unwrap_or(0.0))
                .collect();
            let w = winners(&sv, true);
            rows.push((
                "Solved",
                parts
                    .iter()
                    .enumerate()
                    .map(|(k, p)| CellSpec {
                        text: thousands(p.solve.map(|s| s.solved).unwrap_or(0)),
                        color: None,
                        win: multi && w[k],
                    })
                    .collect(),
            ));
            let av: Vec<f64> = parts
                .iter()
                .map(|p| p.solve.map(|s| pct(s.ok, s.subs)).unwrap_or(0.0))
                .collect();
            let w2 = winners(&av, true);
            rows.push((
                "Acceptance",
                parts
                    .iter()
                    .enumerate()
                    .map(|(k, p)| {
                        let v = p.solve.map(|s| pct(s.ok, s.subs)).unwrap_or(0.0);
                        CellSpec {
                            text: format!("{v:.1}%"),
                            color: None,
                            win: multi && w2[k],
                        }
                    })
                    .collect(),
            ));
        }

        let header_cells = parts
            .iter()
            .map(|p| (PALETTE[p.slot], p.handle.clone()))
            .collect::<Vec<_>>()
            .into_iter()
            .map(|(col, handle)| {
                let href = format!("https://codeforces.com/profile/{handle}");
                view! {
                    <TableHeaderCell>
                        <a
                            href=href
                            target="_blank"
                            rel="noopener"
                            style=format!("color:{col};font-weight:700;text-decoration:none;")
                        >
                            {handle}
                        </a>
                    </TableHeaderCell>
                }
            })
            .collect_view();

        view! {
            <>
                <SectionHeader title="Rating history overlay".into()/>
                <div style="overflow-x:auto;border:1px solid rgba(128,128,128,0.25);border-radius:8px;padding:6px;background:rgba(128,128,128,0.06);">
                    <MultiRatingChart series/>
                </div>

                <SectionHeader title="Head-to-head".into()/>
                <div style="overflow-x:auto;border:1px solid rgba(128,128,128,0.25);border-radius:8px;padding:6px;background:rgba(128,128,128,0.06);">
                    <Table>
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCell>"Metric"</TableHeaderCell>
                                {header_cells}
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            {rows
                                .into_iter()
                                .map(|(label, cells)| metric_row(label, cells))
                                .collect_view()}
                        </TableBody>
                    </Table>
                </div>
            </>
        }
        .into_any()
    };

    view! {
        <Flex vertical=true gap=FlexGap::Medium>
            <SectionHeader title="Compare users".into()/>
            <Flex gap=FlexGap::Medium style="flex-wrap:wrap;">
                <div style="min-width:220px;flex:1;">
                    <Input
                        placeholder="User A handle"
                        value=a_input
                        on:keydown=on_enter
                        input_style="width:100%;"
                    />
                    {caption(0)}
                </div>
                <div style="min-width:220px;flex:1;">
                    <Input
                        placeholder="User B handle"
                        value=b_input
                        on:keydown=on_enter
                        input_style="width:100%;"
                    />
                    {caption(1)}
                </div>
                {move || {
                    third_enabled.get().then(|| view! {
                        <div style="min-width:220px;flex:1;">
                            <Input
                                placeholder="User C handle"
                                value=c_input
                                on:keydown=on_enter
                                input_style="width:100%;"
                            />
                            {caption(2)}
                        </div>
                    })
                }}
            </Flex>
            <Flex gap=FlexGap::Medium style="flex-wrap:wrap;" align=FlexAlign::Center>
                <Checkbox checked=third_enabled label="Add a third user"/>
                <Checkbox checked=load_solves label="Load solve counts (slower)"/>
                <Button appearance=ButtonAppearance::Primary on:click=move |_| compare()>
                    "Compare"
                </Button>
            </Flex>
            <ErrorBar message=ctrl_err/>

            <Flex gap=FlexGap::Small align=FlexAlign::Center style="flex-wrap:wrap;">
                {move || {
                    let link = share_link.get();
                    view! { <CopyLinkButton text=link/> }
                }}
                {move || {
                    ran.get().then(|| {
                        let csv = csv_content.get();
                        view! {
                            <DownloadButton filename="comparison.csv" content=csv>
                                "Export CSV"
                            </DownloadButton>
                        }
                    })
                }}
            </Flex>

            {results}
        </Flex>
    }
}
