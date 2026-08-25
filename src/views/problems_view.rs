//! Problems tab: full Codeforces problem set with dual-tag, rating and name
//! filters, a difficulty histogram, random picker, solved-count bars and
//! configurable pagination. Reads the shared problemset cache from the store.

use crate::api;
use crate::components::*;
use crate::storage;
use crate::store;
use crate::util::*;
use leptos::prelude::*;
use thaw::*;

fn rating_options() -> Vec<String> {
    let mut v: Vec<String> = (800..=3500).step_by(100).map(|r| r.to_string()).collect();
    v.insert(0, String::new());
    v
}

fn page_size_options() -> Vec<usize> {
    vec![25, 50, 100, 200]
}

#[component]
pub fn ProblemsView() -> impl IntoView {
    let shared = store::problemset();

    // Local mirror of the shared problemset, filled once when the shared
    // signal first reports Ready, so the Memo pipeline below stays simple.
    let problems = RwSignal::new(Vec::<api::Problem>::new());
    let error = RwSignal::new(String::new());

    // Filters ---------------------------------------------------------------
    let tag_filter = RwSignal::new(String::new());
    let tag_filter_b = RwSignal::new(String::new());
    let min_rating = RwSignal::new(String::new());
    let max_rating = RwSignal::new(String::new());
    let name_filter = RwSignal::new(String::new());
    let sort_by_solved = RwSignal::new(true);
    let page_size = RwSignal::new(String::from("50"));
    let page = RwSignal::new(1usize);
    let picked = RwSignal::new(None::<api::Problem>);

    Effect::new(move |_| match shared.get() {
        store::SharedProblemset::Ready(arc) => problems.set((*arc).clone()),
        store::SharedProblemset::Error(e) => error.set(e),
        store::SharedProblemset::Loading => {}
    });

    let page_size_n = move || page_size.get().parse::<usize>().unwrap_or(50);

    let all_tags = Memo::new(move |_| {
        let mut tags: Vec<String> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for p in problems.get() {
            for t in p.tags {
                if seen.insert(t.clone()) {
                    tags.push(t);
                }
            }
        }
        tags.sort();
        tags
    });

    let filtered = Memo::new(move |_| {
        let needle = name_filter.get().to_lowercase();
        let tag_a = tag_filter.get();
        let tag_b = tag_filter_b.get();
        let min_r: Option<i32> = min_rating.get().parse().ok();
        let max_r: Option<i32> = max_rating.get().parse().ok();
        let by_solved = sort_by_solved.get();

        let mut list: Vec<api::Problem> = problems
            .get()
            .into_iter()
            .filter(|p| needle.is_empty() || p.name.to_lowercase().contains(&needle))
            .filter(|p| tag_a.is_empty() || p.tags.iter().any(|t| t.eq_ignore_ascii_case(&tag_a)))
            .filter(|p| tag_b.is_empty() || p.tags.iter().any(|t| t.eq_ignore_ascii_case(&tag_b)))
            .filter(|p| {
                let r = p.rating.unwrap_or(0);
                min_r.is_none_or(|m| r >= m) && max_r.is_none_or(|m| r <= m || r == 0)
            })
            .collect();

        if by_solved {
            list.sort_by_key(|p| -p.solved_count);
        } else {
            list.sort_by(|a, b| {
                a.rating
                    .unwrap_or(9999)
                    .cmp(&b.rating.unwrap_or(9999))
                    .then_with(|| a.solved_count.cmp(&b.solved_count).reverse())
            });
        }
        list
    });

    let page_count = Memo::new(move |_| {
        let ps = page_size_n();
        (filtered.get().len() + ps - 1).max(1) / ps
    });

    // Any filter (or page-size) change resets pagination.
    Effect::new(move |_| {
        name_filter.track();
        tag_filter.track();
        tag_filter_b.track();
        min_rating.track();
        max_rating.track();
        sort_by_solved.track();
        page_size.track();
        page.set(1);
    });

    let paged = Memo::new(move |_| {
        let ps = page_size_n();
        let rows: Vec<api::Problem> = filtered
            .get()
            .into_iter()
            .skip((page.get().saturating_sub(1)) * ps)
            .take(ps)
            .collect();
        // Relative solved-count bar widths within this page.
        let max_solved = rows
            .iter()
            .map(|p| p.solved_count)
            .max()
            .unwrap_or(0)
            .max(1);
        rows.into_iter()
            .map(|p| {
                let w = (p.solved_count as f64 / max_solved as f64 * 100.0).clamp(0.0, 100.0);
                (p, w)
            })
            .collect::<Vec<_>>()
    });

    // Difficulty histogram over the filtered set.
    let hist = Memo::new(move |_| {
        let list = filtered.get();
        (800..3400)
            .step_by(200)
            .map(|bucket| {
                let count = list
                    .iter()
                    .filter(|p| p.rating.is_some_and(|r| r >= bucket && r < bucket + 200))
                    .count() as i64;
                (bucket.to_string(), count, rating_color(bucket + 100))
            })
            .collect::<Vec<_>>()
    });

    let pick_random = move |_| {
        picked.set(None);
        let len = filtered.with_untracked(|f| f.len());
        if len > 0 {
            let idx = (storage::epoch_ms() as usize) % len;
            picked.set(Some(filtered.with_untracked(|f| f[idx].clone())));
        }
    };

    view! {
        <Flex vertical=true gap=FlexGap::Medium>
            <SectionHeader title="Problem set".into()/>

            <Flex gap=FlexGap::Small align=FlexAlign::End style="flex-wrap:wrap;">
                <Field label="Name contains">
                    <Input placeholder="e.g. binary search" value=name_filter input_style="width:200px;"/>
                </Field>
                <Field label="Tag">
                    {move || view! {
                        <Select default_value="" value=tag_filter>
                            <option value="">"Any tag"</option>
                            {all_tags.get().into_iter().map(|t| {
                                let label = t.clone();
                                view! { <option value=t>{label}</option> }
                            }).collect_view()}
                        </Select>
                    }}
                </Field>
                <Field label="Tag (AND)">
                    {move || view! {
                        <Select default_value="" value=tag_filter_b>
                            <option value="">"Any tag"</option>
                            {all_tags.get().into_iter().map(|t| {
                                let label = t.clone();
                                view! { <option value=t>{label}</option> }
                            }).collect_view()}
                        </Select>
                    }}
                </Field>
                <Field label="Min rating">
                    <Select default_value="" value=min_rating>
                        {rating_options().into_iter().map(|r| {
                            let label = if r.is_empty() { "any".to_string() } else { r.clone() };
                            view! { <option value=r>{label}</option> }
                        }).collect_view()}
                    </Select>
                </Field>
                <Field label="Max rating">
                    <Select default_value="" value=max_rating>
                        {rating_options().into_iter().map(|r| {
                            let label = if r.is_empty() { "any".to_string() } else { r.clone() };
                            view! { <option value=r>{label}</option> }
                        }).collect_view()}
                    </Select>
                </Field>
                <Field label="Per page">
                    <Select default_value="50" value=page_size>
                        {page_size_options().into_iter().map(|n| {
                            view! { <option value=n.to_string()>{n}</option> }
                        }).collect_view()}
                    </Select>
                </Field>
                <Checkbox checked=sort_by_solved label="Sort by solved"/>
                <Button appearance=ButtonAppearance::Secondary on:click=pick_random>
                    "Pick random"
                </Button>
            </Flex>

            {move || -> AnyView {
                match shared.get() {
                    store::SharedProblemset::Loading => {
                        view! { <Loading label="Loading the entire problem pool".into()/> }.into_any()
                    }
                    store::SharedProblemset::Error(_) => {
                        view! { <ErrorBar message=error/> }.into_any()
                    }
                    store::SharedProblemset::Ready(_) => view! {
                        <>
                            {move || {
                                picked.get().map(|p| {
                                    let code = p.code();
                                    let link = p.url();
                                    let pname = truncate(&p.name, 90);
                                    let rating = p.rating;
                                    let solved = thousands(p.solved_count as i64);
                                    view! {
                                        <Card>
                                            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center style="flex-wrap:wrap;gap:8px;">
                                                <div>
                                                    <b>{code}</b>
                                                    " "
                                                    {rating.map(|r| view! { <RatingPill rating=r/> })}
                                                    <Caption1>" \u{00b7} solved by "{solved}</Caption1>
                                                    <p style="margin:6px 0 0;">
                                                        <a href=link target="_blank" rel="noopener" style="font-weight:600;">{pname}</a>
                                                    </p>
                                                </div>
                                                <Button appearance=ButtonAppearance::Subtle on:click=pick_random>
                                                    "Roll again"
                                                </Button>
                                            </Flex>
                                        </Card>
                                    }
                                })
                            }}

                            {move || {
                                let bars = hist.get();
                                bars.iter().any(|b| b.1 > 0)
                                    .then(|| view! { <Histogram bars/> })
                            }}

                            <Caption1>{format!("{} problems matched", thousands(filtered.with(|f| f.len()) as i64))}</Caption1>
                            <div style="overflow-x:auto;border:1px solid rgba(128,128,128,0.25);border-radius:8px;padding:4px;">
                                <Table>
                                    <TableHeader>
                                        <TableRow>
                                            <TableHeaderCell>"Code"</TableHeaderCell>
                                            <TableHeaderCell>"Name"</TableHeaderCell>
                                            <TableHeaderCell>"Rating"</TableHeaderCell>
                                            <TableHeaderCell>"Solved"</TableHeaderCell>
                                            <TableHeaderCell>"Tags"</TableHeaderCell>
                                        </TableRow>
                                    </TableHeader>
                                    <TableBody>
                                        {move || paged.get().into_iter().map(|(p, w)| {
                                            let code = p.code();
                                            let link = p.url();
                                            let rating = p.rating;
                                            let solved = thousands(p.solved_count as i64);
                                            let bar = format!("width:{w:.0}%;height:3px;background:#0078d4;opacity:.55;border-radius:2px;margin-top:2px;");
                                            let tags = truncate(&p.tags.join(", "), 80);
                                            let name = truncate(&p.name, 70);
                                            view! {
                                                <TableRow>
                                                    <TableCell>{code}</TableCell>
                                                    <TableCell><a href=link target="_blank" rel="noopener">{name}</a></TableCell>
                                                    <TableCell>
                                                        {rating.map(|r| view! {
                                                            <b style=format!("color:{};", rating_color(r))>{r}</b>
                                                        })}
                                                    </TableCell>
                                                    <TableCell>
                                                        <div>{solved}</div>
                                                        <div style=bar></div>
                                                    </TableCell>
                                                    <TableCell><Caption1>{tags}</Caption1></TableCell>
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
        </Flex>
    }
}
