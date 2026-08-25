//! Problems tab: full Codeforces problem set with tag, rating and name
//! filters, sortable-ish listing and pagination.

use crate::api;
use crate::components::*;
use crate::util::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

const PAGE_SIZE: usize = 50;

fn rating_options() -> Vec<String> {
    let mut v: Vec<String> = (800..=3500).step_by(100).map(|r| r.to_string()).collect();
    v.insert(0, String::new());
    v
}

#[component]
pub fn ProblemsView() -> impl IntoView {
    let problems = RwSignal::new(Vec::<api::Problem>::new());
    let loading = RwSignal::new(true);
    let error = RwSignal::new(String::new());

    // Filters ---------------------------------------------------------------
    let tag_filter = RwSignal::new(String::new());
    let min_rating = RwSignal::new(String::new());
    let max_rating = RwSignal::new(String::new());
    let name_filter = RwSignal::new(String::new());
    let sort_by_solved = RwSignal::new(true);
    let page = RwSignal::new(1usize);

    spawn_local(async move {
        match api::problemset_problems(&[]).await {
            Ok(res) => problems.set(res.problems),
            Err(e) => error.set(e),
        }
        loading.set(false);
    });

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
        let tag = tag_filter.get();
        let min_r: Option<i32> = min_rating.get().parse().ok();
        let max_r: Option<i32> = max_rating.get().parse().ok();
        let by_solved = sort_by_solved.get();

        let mut list: Vec<api::Problem> = problems
            .get()
            .into_iter()
            .filter(|p| needle.is_empty() || p.name.to_lowercase().contains(&needle))
            .filter(|p| tag.is_empty() || p.tags.iter().any(|t| t.eq_ignore_ascii_case(&tag)))
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

    let page_count = Memo::new(move |_| (filtered.get().len() + PAGE_SIZE - 1).max(1) / PAGE_SIZE);

    // Any filter change resets pagination.
    Effect::new(move |_| {
        name_filter.track();
        tag_filter.track();
        min_rating.track();
        max_rating.track();
        sort_by_solved.track();
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
                <Checkbox checked=sort_by_solved label="Sort by solved"/>
            </Flex>

            {move || -> AnyView {
                if loading.get() {
                    view! { <Loading label="Loading the entire problem set (this can take a few seconds)".into()/> }.into_any()
                } else if !error.get().is_empty() {
                    view! { <ErrorBar message=error/> }.into_any()
                } else {
                    view! {
                        <>
                            <Caption1>{format!("{} problems matched", thousands(filtered.with(|f| f.len()) as i64))}</Caption1>
                            <div style="overflow-x:auto;border:1px solid #eee;border-radius:8px;padding:4px;">
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
                                        {move || paged.get().into_iter().map(|p| {
                                            let code = p.code();
                                            let link = p.url();
                                            let rating = p.rating;
                                            let solved = p.solved_count;
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
                                                    <TableCell>{thousands(solved as i64)}</TableCell>
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
