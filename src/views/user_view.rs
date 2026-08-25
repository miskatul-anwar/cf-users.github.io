//! User tab: profile lookup (multiple handles), solve analytics, practice
//! recommendations, rating history, recent submissions and blog entries.

use crate::api;
use crate::components::*;
use crate::storage;
use crate::store;
use crate::util::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::{HashMap, HashSet};
use thaw::*;

#[component]
pub fn UserView(initial_handle: Option<String>) -> impl IntoView {
    // Inputs & data ----------------------------------------------------------
    let handle_input = RwSignal::new(String::new());
    let users = RwSignal::new(Vec::<api::User>::new());
    let rating_changes = RwSignal::new(Vec::<api::RatingChange>::new());
    let submissions = RwSignal::new(Vec::<api::Submission>::new());
    let analytics_subs = RwSignal::new(Vec::<api::Submission>::new());
    let solved_keys = RwSignal::new(None::<HashSet<String>>);
    let blogs = RwSignal::new(Vec::<api::BlogEntry>::new());

    let sub_count = RwSignal::new(String::from("50"));

    // Per-section loading / error: 0 info, 1 rating, 2 submissions, 3 blogs,
    // 4 solve analytics.
    let loading = RwSignal::new([false; 5]);
    let set_loading = move |i: usize, v: bool| loading.update(|a| a[i] = v);
    let err_info = RwSignal::new(String::new());
    let err_rating = RwSignal::new(String::new());
    let err_subs = RwSignal::new(String::new());
    let err_blogs = RwSignal::new(String::new());
    let err_analytics = RwSignal::new(String::new());

    let last_handles = RwSignal::new(Vec::<String>::new());
    let last_primary = RwSignal::new(None::<String>);

    let search = move || {
        let handles = parse_handles(&handle_input.get_untracked());
        if handles.is_empty() {
            err_info.set("Please enter at least one Codeforces handle.".into());
            return;
        }
        err_info.set(String::new());
        err_rating.set(String::new());
        err_subs.set(String::new());
        err_blogs.set(String::new());
        err_analytics.set(String::new());
        last_handles.set(handles.clone());

        // user.info supports multiple handles in one request.
        set_loading(0, true);
        users.set(Vec::new());
        let hs = handles.clone();
        spawn_local(async move {
            match api::user_info(&hs.iter().map(String::as_str).collect::<Vec<_>>()).await {
                Ok(u) => users.set(u),
                Err(e) => err_info.set(e),
            }
            set_loading(0, false);
        });

        // Remaining endpoints operate on the first handle.
        let primary = handles[0].clone();
        last_primary.set(Some(primary.clone()));

        set_loading(1, true);
        rating_changes.set(Vec::new());
        let h = primary.clone();
        spawn_local(async move {
            match api::user_rating_cached(&h).await {
                Ok(c) => rating_changes.set(c),
                Err(e) => err_rating.set(e),
            }
            set_loading(1, false);
        });

        set_loading(2, true);
        submissions.set(Vec::new());
        let cnt: u32 = sub_count.get_untracked().parse().unwrap_or(50);
        let h = primary.clone();
        spawn_local(async move {
            match api::user_status_cached(&h, cnt).await {
                Ok(s) => submissions.set(s),
                Err(e) => err_subs.set(e),
            }
            set_loading(2, false);
        });

        set_loading(3, true);
        blogs.set(Vec::new());
        let h = primary.clone();
        spawn_local(async move {
            match api::user_blog_entries(&h).await {
                Ok(b) => blogs.set(b),
                Err(e) => err_blogs.set(e),
            }
            set_loading(3, false);
        });

        set_loading(4, true);
        analytics_subs.set(Vec::new());
        solved_keys.set(None);
        let h = primary;
        spawn_local(async move {
            match api::user_status_cached(&h, 2000).await {
                Ok(s) => {
                    solved_keys.set(Some(collect_solved_keys(&s)));
                    analytics_subs.set(s);
                }
                Err(e) => err_analytics.set(e),
            }
            set_loading(4, false);
        });
    };

    // Deep links like #/user/tourist pre-fill the input and search once.
    if let Some(h) = initial_handle {
        handle_input.set(h.trim().to_string());
        search();
    }

    let on_keydown = move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Enter" {
            search();
        }
    };

    // Reload submissions when the count selector changes.
    Effect::new(move |_| {
        let cnt: u32 = sub_count.get().parse().unwrap_or(50);
        let handles = last_handles.get_untracked();
        if handles.is_empty() || loading.get_untracked()[2] {
            return;
        }
        err_subs.set(String::new());
        set_loading(2, true);
        spawn_local(async move {
            match api::user_status_cached(&handles[0], cnt).await {
                Ok(s) => submissions.set(s),
                Err(e) => err_subs.set(e),
            }
            set_loading(2, false);
        });
    });

    // Shareable profile deep link (never mutates the URL directly).
    let share_link = Memo::new(move |_| {
        let href = window().location().href().unwrap_or_default();
        let base = href.split('#').next().unwrap_or_default().to_string();
        match last_primary.get() {
            Some(h) => format!("{base}#/user/{}", h.replace('/', "")),
            None => base,
        }
    });

    // Live rating of the primary handle, once user.info answers.
    let current_rating = Memo::new(move |_| {
        let primary = last_primary.get()?;
        users
            .get()
            .into_iter()
            .find(|u| u.handle.eq_ignore_ascii_case(&primary))
            .and_then(|u| u.rating)
    });

    // Blog entry dialog (uses blogEntry.view + blogEntry.comments) ----------
    let blog_open = RwSignal::new(false);
    let blog_title = RwSignal::new(String::new());
    let blog_body = RwSignal::new(String::from("Loading\u{2026}"));
    let blog_link = RwSignal::new(String::new());
    let blog_comment_list = RwSignal::new(Vec::<api::Comment>::new());
    let blog_err = RwSignal::new(String::new());

    let open_blog = move |id: i64, fallback_title: String| {
        blog_title.set(fallback_title);
        blog_body.set("Loading\u{2026}".into());
        blog_comment_list.set(Vec::new());
        blog_err.set(String::new());
        blog_link.set(format!("https://codeforces.com/blog/entry/{id}"));
        blog_open.set(true);
        spawn_local(async move {
            match api::blog_entry_view(id).await {
                Ok(b) => {
                    blog_title.set(b.title.clone());
                    blog_body.set(strip_html(b.content.as_deref().unwrap_or("(no content)")));
                }
                Err(e) => blog_err.set(e),
            }
            if let Ok(cs) = api::blog_entry_comments(id).await {
                blog_comment_list.set(cs);
            }
        });
    };

    // Views ------------------------------------------------------------------
    let show_sections = Memo::new(move |_| !last_handles.get().is_empty());

    let user_cards = move || {
        users
            .get()
            .into_iter()
            .map(|u| {
                let handle = u.handle.clone();
                let profile = format!("https://codeforces.com/profile/{handle}");
                let rank = u.rank.clone().unwrap_or_default();
                let rc = rank_color(&rank);
                let max_rank = u.max_rank.clone().unwrap_or_default();
                let mrc = rank_color(&max_rank);
                let rating = u.rating.unwrap_or(0);
                let max_rating = u.max_rating.unwrap_or(0);
                let contribution = u.contribution;
                let cc = if contribution >= 0 { "#008000" } else { "#ff0000" };
                let org = u.organization.unwrap_or_else(|| "N/A".into());
                let loc = match (u.country.clone(), u.city.clone()) {
                    (Some(c), Some(city)) => format!("{c}, {city}"),
                    (Some(c), None) => c,
                    (None, Some(city)) => city,
                    _ => "N/A".into(),
                };
                let registered = format!(
                    "{} ({})",
                    format_date(u.registration_time_seconds),
                    member_since(u.registration_time_seconds, storage::now_secs())
                );
                let online = format_time(u.last_online_time_seconds);
                let friends = thousands(u.friend_of_count as i64);
                let photo = if u.title_photo.is_empty() {
                    "https://userpic.codeforces.org/no-title.jpg".to_string()
                } else {
                    u.title_photo.clone()
                };
                let rating_s =
                    if rating > 0 { rating.to_string() } else { "\u{2014}".to_string() };
                let max_pill: AnyView = if max_rating > 0 {
                    view! { <RatingPill rating=max_rating/> }.into_any()
                } else {
                    view! { <span>"\u{2014}"</span> }.into_any()
                };
                let chip: AnyView = if rank.is_empty() {
                    view! { <Caption1 style="opacity:0.7;">"unrated"</Caption1> }.into_any()
                } else {
                    view! {
                        <span style=format!(
                            "align-self:flex-start;background:{rc}22;color:{rc};padding:1px 10px;border-radius:999px;font-weight:600;font-size:0.85rem;",
                        )>{rank}</span>
                    }
                    .into_any()
                };
                view! {
                    <div style="min-width:300px;flex:1;">
                        <Card>
                            <Flex vertical=true gap=FlexGap::Small>
                                <img
                                    src=photo
                                    style="width:100%;border-radius:8px;border:1px solid rgba(128,128,128,0.25);"
                                    alt="avatar"
                                />
                                <Text style="font-size:1.2em;font-weight:700;">
                                    <a
                                        href=profile
                                        target="_blank"
                                        rel="noopener"
                                        style=format!("color:{rc};text-decoration:none;")
                                    >
                                        {handle.clone()}
                                    </a>
                                </Text>
                                {chip}
                                <Caption1 style=format!("color:{mrc};font-weight:600;")>
                                    {format!("Max: {max_rank}")}
                                </Caption1>
                                <Divider/>
                                <p><b>"Rating: "</b><b style=format!("color:{rc};")>{rating_s}</b></p>
                                <p><b>"Max rating: "</b>{max_pill}</p>
                                <p>
                                    <b>"Contribution: "</b>
                                    <b style=format!("color:{cc};")>{signed_delta(contribution)}</b>
                                </p>
                                <p><b>"Organization: "</b>{org}</p>
                                <p><b>"Location: "</b>{loc}</p>
                                <p><b>"Registered: "</b>{registered}</p>
                                <p><b>"Last online: "</b>{online}</p>
                                <p><b>"Friends of: "</b>{friends}</p>
                            </Flex>
                        </Card>
                    </div>
                }
            })
            .collect_view()
    };

    let analytics_section = move || -> AnyView {
        if loading.get()[4] {
            view! { <Loading label="Loading solve analytics".into()/> }.into_any()
        } else if !err_analytics.get().is_empty() {
            view! { <ErrorBar message=err_analytics/> }.into_any()
        } else if analytics_subs.with(|s| s.is_empty()) {
            view! { <Empty text="No submissions found for this user.".into()/> }.into_any()
        } else {
            let subs = analytics_subs.get_untracked();
            let total = subs.len() as i64;
            let mut solved: HashSet<String> = HashSet::new();
            let mut attempted: HashSet<String> = HashSet::new();
            let mut solved_map: HashMap<String, api::Problem> = HashMap::new();
            let mut daily: HashMap<i64, u32> = HashMap::new();
            let mut verdicts: HashMap<String, i64> = HashMap::new();
            let mut langs: HashMap<String, i64> = HashMap::new();
            let mut ac_days: Vec<i64> = Vec::new();
            let mut ac_total = 0_i64;

            for s in &subs {
                let key = problem_key(s.problem.contest_id, &s.problem.index);
                attempted.insert(key.clone());
                let label = verdict_text(s.verdict.as_deref().unwrap_or("TESTING"));
                *verdicts.entry(label).or_insert(0) += 1;
                *langs.entry(s.programming_language.clone()).or_insert(0) += 1;
                if s.verdict.as_deref() == Some("OK") {
                    ac_total += 1;
                    solved.insert(key.clone());
                    solved_map.insert(key, s.problem.clone());
                    let day = day_of(s.creation_time_seconds);
                    *daily.entry(day).or_insert(0) += 1;
                    ac_days.push(day);
                }
            }
            ac_days.sort_unstable();
            ac_days.dedup();
            let today = day_of(storage::now_secs());
            let (longest_streak, current_streak) = streak_of(&ac_days, today);

            let daily_vec: Vec<(i64, u32)> = daily.into_iter().collect();
            let verdict_items = top_counts(verdicts, 8);
            let lang_items = top_counts(langs, 6);
            let mut tag_counts: HashMap<String, i64> = HashMap::new();
            for p in solved_map.values() {
                for t in &p.tags {
                    *tag_counts.entry(t.clone()).or_insert(0) += 1;
                }
            }
            let tag_items = top_counts(tag_counts, 10);
            let ratings: Vec<i32> = solved_map.values().filter_map(|p| p.rating).collect();
            let bars = difficulty_bars(&ratings);

            let total_s = thousands(total);
            let solved_s = thousands(solved.len() as i64);
            let attempted_s = thousands(attempted.len() as i64);
            let acc_s = format!("{:.1}", pct(ac_total, total));
            let longest_s = longest_streak.to_string();
            let current_s = current_streak.to_string();

            view! {
                <Flex vertical=true gap=FlexGap::Medium>
                    <Flex gap=FlexGap::Large style="flex-wrap:wrap;">
                        <Stat label="Submissions" value=total_s/>
                        <Stat label="Solved" value=solved_s color="#008000"/>
                        <Stat label="Attempted" value=attempted_s/>
                        <Stat label="Acceptance %" value=acc_s color="#0078d4"/>
                        <Stat label="Longest streak (days)" value=longest_s/>
                        <Stat label="Current streak (days)" value=current_s color="#0078d4"/>
                    </Flex>
                    <Heatmap daily=daily_vec/>
                    <Flex gap=FlexGap::Large style="flex-wrap:wrap;">
                        <div style="flex:1;min-width:280px;">
                            <Text style="font-weight:600;">"Verdict distribution"</Text>
                            <NamedCountBar items=verdict_items color="#0078d4"/>
                        </div>
                        <div style="flex:1;min-width:280px;">
                            <Text style="font-weight:600;">"Top languages"</Text>
                            <NamedCountBar items=lang_items color="#aa00aa"/>
                        </div>
                        <div style="flex:1;min-width:280px;">
                            <Text style="font-weight:600;">"Tags of solved problems"</Text>
                            <NamedCountBar items=tag_items color="#008000"/>
                        </div>
                    </Flex>
                    <div>
                        <Text style="font-weight:600;">"Solved difficulty"</Text>
                        <Histogram bars=bars/>
                    </div>
                </Flex>
            }
            .into_any()
        }
    };

    let recommendations_section = move || -> AnyView {
        match store::problemset().get() {
            store::SharedProblemset::Loading => {
                view! { <Loading label="Loading problem pool".into()/> }.into_any()
            }
            store::SharedProblemset::Error(e) => {
                let msg = RwSignal::new(e);
                view! { <ErrorBar message=msg/> }.into_any()
            }
            store::SharedProblemset::Ready(problems) => match solved_keys.get() {
                None if loading.get_untracked()[4] => {
                    view! { <Loading label="Loading solve history".into()/> }.into_any()
                }
                None => {
                    view! { <Empty text="Solve history unavailable for recommendations.".into()/> }
                        .into_any()
                }
                Some(solved) => {
                    let (lo, hi) = match current_rating.get() {
                        Some(r) => (r.saturating_sub(100).max(800), r + 300),
                        None => (1000, 1600),
                    };
                    let mut cands: Vec<&api::Problem> = problems
                        .iter()
                        .filter(|p| p.rating.is_some_and(|r| r >= lo && r <= hi))
                        .filter(|p| !solved.contains(&problem_key(p.contest_id, &p.index)))
                        .collect();
                    cands.sort_by_key(|a| std::cmp::Reverse(a.solved_count));
                    cands.truncate(10);
                    if cands.is_empty() {
                        view! { <Empty text="No unsolved problems found in this rating band.".into()/> }
                            .into_any()
                    } else {
                        let band = format!("Recommended difficulty range: {lo}\u{2013}{hi}");
                        let rows = cands
                            .into_iter()
                            .map(|p| {
                                let name = truncate(&p.name, 60);
                                let url = p.url();
                                let r = p.rating.unwrap_or(0);
                                let by = thousands(p.solved_count as i64);
                                view! {
                                    <div style="display:flex;align-items:center;gap:10px;padding:6px 10px;border:1px solid rgba(128,128,128,0.25);border-radius:8px;background:rgba(128,128,128,0.06);flex-wrap:wrap;">
                                        <a href=url target="_blank" rel="noopener">{name}</a>
                                        <RatingPill rating=r/>
                                        <Caption1 style="margin-left:auto;opacity:0.8;">
                                            {format!("solved by {by}")}
                                        </Caption1>
                                    </div>
                                }
                            })
                            .collect_view();
                        view! {
                            <Flex vertical=true gap=FlexGap::Small>
                                <Caption1>{band}</Caption1>
                                {rows}
                            </Flex>
                        }
                        .into_any()
                    }
                }
            },
        }
    };

    let rating_section = move || -> AnyView {
        if loading.get()[1] {
            view! { <Loading label="Loading rating history".into()/> }.into_any()
        } else if !err_rating.get().is_empty() {
            view! { <ErrorBar message=err_rating/> }.into_any()
        } else if rating_changes.with(|c| c.is_empty()) {
            view! { <Empty text="No rating history for this user.".into()/> }.into_any()
        } else {
            let changes = rating_changes.get_untracked();
            let best_rank = changes.iter().map(|c| c.rank).min().unwrap_or(0);
            let best_rating = changes.iter().map(|c| c.new_rating).max().unwrap_or(0);
            let gains: Vec<i32> = changes
                .iter()
                .map(|c| c.new_rating - c.old_rating)
                .collect();
            let total_delta: i32 = gains.iter().sum();
            let best_delta = *gains.iter().max().unwrap_or(&0);
            let net_color: &str = if total_delta >= 0 {
                "#008000"
            } else {
                "#ff0000"
            };
            let jump_color: &str = if best_delta >= 0 {
                "#008000"
            } else {
                "#ff0000"
            };
            let n_contests = changes.len().to_string();
            let best_rank_s = best_rank.to_string();
            let best_rating_s = best_rating.to_string();
            view! {
                <Flex vertical=true gap=FlexGap::Medium>
                    <Flex gap=FlexGap::Large style="flex-wrap:wrap;">
                        <Stat label="Contests" value=n_contests/>
                        <Stat label="Best place" value=best_rank_s/>
                        <Stat label="Peak rating" value=best_rating_s color=rating_color(best_rating)/>
                        <Stat label="Net change" value=signed_delta(total_delta) color=net_color/>
                        <Stat label="Best jump" value=signed_delta(best_delta) color=jump_color/>
                    </Flex>
                    <RatingChart changes=changes.clone()/>
                    <div style="max-height:420px;overflow-y:auto;border:1px solid rgba(128,128,128,0.25);border-radius:8px;padding:4px;">
                        <RatingChangeTable changes=changes show_handle=false/>
                    </div>
                </Flex>
            }
            .into_any()
        }
    };

    let subs_section = move || -> AnyView {
        if loading.get()[2] {
            view! { <Loading label="Loading submissions".into()/> }.into_any()
        } else if !err_subs.get().is_empty() {
            view! { <ErrorBar message=err_subs/> }.into_any()
        } else if submissions.with(|s| s.is_empty()) {
            view! { <Empty text="No submissions found.".into()/> }.into_any()
        } else {
            let subs = submissions.get_untracked();
            let csv = submissions_csv(&subs);
            view! {
                <Flex vertical=true gap=FlexGap::Small>
                    <Flex justify=FlexJustify::End>
                        <DownloadButton filename="submissions.csv" content=csv>"Export CSV"</DownloadButton>
                    </Flex>
                    <SubmissionTable subs=subs/>
                </Flex>
            }
            .into_any()
        }
    };

    let blogs_section = move || -> AnyView {
        if loading.get()[3] {
            view! { <Loading label="Loading blogs".into()/> }.into_any()
        } else if !err_blogs.get().is_empty() {
            view! { <ErrorBar message=err_blogs/> }.into_any()
        } else if blogs.with(|b| b.is_empty()) {
            view! { <Empty text="No blog entries found.".into()/> }.into_any()
        } else {
            view! { <BlogList blogs=blogs.get_untracked() on_open=open_blog/> }.into_any()
        }
    };

    view! {
        <Flex vertical=true gap=FlexGap::Medium>
            <SectionHeader title="User lookup".into()/>
            <Flex gap=FlexGap::Small align=FlexAlign::Center style="flex-wrap:wrap;">
                <Input
                    placeholder="Handle (use ; for several)"
                    value=handle_input
                    on:keydown=on_keydown
                    input_style="min-width:280px;"
                />
                <Button appearance=ButtonAppearance::Primary on:click=move |_| search()>
                    "Search"
                </Button>
                {move || {
                    let link = share_link.get();
                    view! { <CopyLinkButton text=link/> }
                }}
            </Flex>
            <ErrorBar message=err_info/>

            {move || {
                (!users.get().is_empty()).then(|| view! {
                    <Flex gap=FlexGap::Medium style="flex-wrap:wrap;">
                        {user_cards()}
                    </Flex>
                })
            }}

            {move || show_sections.get().then(|| view! {
                <>
                    <SectionHeader title="Solve analytics".into()/>
                    {analytics_section()}

                    <SectionHeader title="Practice recommendations".into()/>
                    {recommendations_section()}

                    <SectionHeader title="Rating history".into()/>
                    {rating_section()}

                    <SectionHeader title="Recent submissions".into()/>
                    <Flex gap=FlexGap::Small align=FlexAlign::Center style="flex-wrap:wrap;">
                        <Text>"Show:"</Text>
                        <Select default_value="50" value=sub_count>
                            <option value="25">"25"</option>
                            <option value="50">"50"</option>
                            <option value="100">"100"</option>
                            <option value="250">"250"</option>
                            <option value="1000">"1000"</option>
                        </Select>
                    </Flex>
                    <br/>
                    {subs_section()}

                    <SectionHeader title="Blog entries".into()/>
                    {blogs_section()}
                </>
            })}
        </Flex>

        // Full blog entry dialog
        <Dialog open=blog_open>
            <DialogSurface>
                <DialogBody>
                    <DialogTitle>{blog_title}</DialogTitle>
                    <DialogContent>
                        <Flex vertical=true gap=FlexGap::Medium>
                            <ErrorBar message=blog_err/>
                            <p style="white-space:pre-wrap;max-height:40vh;overflow-y:auto;">{blog_body}</p>
                            <a href=blog_link target="_blank" rel="noopener">"Open on Codeforces"</a>
                            <Divider/>
                            <Text style="font-weight:600;">"Comments"</Text>
                            {move || -> AnyView {
                                let cs = blog_comment_list.get();
                                if cs.is_empty() {
                                    view! { <Caption1>"No comments."</Caption1> }.into_any()
                                } else {
                                    let items = cs.into_iter().map(|c| {
                                        let text = truncate(&strip_html(&c.text), 300);
                                        let who = c.commentator_handle;
                                        let when = format_time(c.creation_time_seconds);
                                        view! {
                                            <div style="border-left:3px solid rgba(128,128,128,0.35);padding-left:10px;margin-bottom:8px;">
                                                <p style="white-space:pre-wrap;">{text}</p>
                                                <Caption1>{who}" \u{00b7} "{when}</Caption1>
                                            </div>
                                        }
                                    }).collect_view();
                                    view! { <div>{items}</div> }.into_any()
                                }
                            }}
                        </Flex>
                    </DialogContent>
                    <DialogActions>
                        <Button on:click=move |_| blog_open.set(false)>"Close"</Button>
                    </DialogActions>
                </DialogBody>
            </DialogSurface>
        </Dialog>
    }
}

#[component]
fn BlogList(
    blogs: Vec<api::BlogEntry>,
    #[prop(into)] on_open: Callback<(i64, String)>,
) -> impl IntoView {
    view! {
        <Table>
            <TableHeader>
                <TableRow>
                    <TableHeaderCell>"Title"</TableHeaderCell>
                    <TableHeaderCell>"Date"</TableHeaderCell>
                    <TableHeaderCell>"Rating"</TableHeaderCell>
                    <TableHeaderCell>"Tags"</TableHeaderCell>
                </TableRow>
            </TableHeader>
            <TableBody>
                {blogs
                    .into_iter()
                    .map(|b| {
                        let id = b.id;
                        let title = b.title.clone();
                        let date = format_date(b.creation_time_seconds);
                        let r = b.rating.unwrap_or(0);
                        let tags = b.tags.join(", ");
                        let display_title = truncate(&b.title, 70);
                        view! {
                            <TableRow>
                                <TableCell>
                                    <a
                                        href=format!("https://codeforces.com/blog/entry/{id}")
                                        target="_blank"
                                        rel="noopener"
                                        style="cursor:pointer;"
                                        on:click=move |ev: leptos::ev::MouseEvent| {
                                            ev.prevent_default();
                                            on_open.run((id, title.clone()));
                                        }
                                    >
                                        {display_title}
                                    </a>
                                </TableCell>
                                <TableCell>{date}</TableCell>
                                <TableCell>
                                    {if r != 0 { format!("{r:+}") } else { "\u{2014}".to_string() }}
                                </TableCell>
                                <TableCell><Caption1>{tags}</Caption1></TableCell>
                            </TableRow>
                        }
                    })
                    .collect_view()}
            </TableBody>
        </Table>
    }
}

// ---------------------------------------------------------------------------
// Pure helpers over submission data
// ---------------------------------------------------------------------------

/// Keys of problems the user has been accepted on at least once.
fn collect_solved_keys(subs: &[api::Submission]) -> HashSet<String> {
    subs.iter()
        .filter(|s| s.verdict.as_deref() == Some("OK"))
        .map(|s| problem_key(s.problem.contest_id, &s.problem.index))
        .collect()
}

/// (longest ever, currently active) streak lengths over sorted unique days.
fn streak_of(days: &[i64], today: i64) -> (i64, i64) {
    let mut longest = 0_i64;
    let mut run = 0_i64;
    let mut prev: Option<i64> = None;
    for &d in days {
        run = if prev == Some(d - 1) { run + 1 } else { 1 };
        longest = longest.max(run);
        prev = Some(d);
    }
    let current = if days.last().is_some_and(|&last| last >= today - 1) {
        run
    } else {
        0
    };
    (longest, current)
}

/// Largest-first slice of a label-count map (ties broken alphabetically).
fn top_counts(counts: HashMap<String, i64>, limit: usize) -> Vec<NamedCount> {
    let mut items: Vec<NamedCount> = counts
        .into_iter()
        .map(|(label, count)| NamedCount { label, count })
        .collect();
    items.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.label.cmp(&b.label)));
    items.truncate(limit);
    items
}

/// Unique-solved counts per 200-point difficulty bucket, 800..3400.
fn difficulty_bars(ratings: &[i32]) -> Vec<(String, i64, &'static str)> {
    (800..3400)
        .step_by(200)
        .map(|lo| {
            let count = ratings.iter().filter(|&&r| r >= lo && r < lo + 200).count() as i64;
            (lo.to_string(), count, rating_color(lo + 100))
        })
        .collect()
}

/// CSV export of the currently listed submissions.
fn submissions_csv(subs: &[api::Submission]) -> String {
    let mut rows: Vec<Vec<String>> = vec![
        [
            "When", "Problem", "Lang", "Verdict", "Tests", "Points", "URL",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
    ];
    for s in subs {
        rows.push(vec![
            storage::csv_cell(&format_time(s.creation_time_seconds)),
            storage::csv_cell(&format!("{} {}", s.problem.code(), s.problem.name)),
            storage::csv_cell(&s.programming_language),
            storage::csv_cell(&verdict_text(s.verdict.as_deref().unwrap_or("TESTING"))),
            s.passed_test_count.to_string(),
            format!("{:.1}", s.points.unwrap_or(0.0)),
            storage::csv_cell(&s.url()),
        ]);
    }
    storage::csv(rows)
}
