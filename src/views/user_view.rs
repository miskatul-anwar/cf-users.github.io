//! User tab: profile lookup (multiple handles), rating history,
//! recent submissions and blog entries.

use crate::api;
use crate::components::*;
use crate::util::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

#[component]
pub fn UserView() -> impl IntoView {
    // Inputs & data ----------------------------------------------------------
    let handle_input = RwSignal::new(String::new());
    let users = RwSignal::new(Vec::<api::User>::new());
    let rating_changes = RwSignal::new(Vec::<api::RatingChange>::new());
    let submissions = RwSignal::new(Vec::<api::Submission>::new());
    let blogs = RwSignal::new(Vec::<api::BlogEntry>::new());

    let sub_count = RwSignal::new(String::from("50"));

    // Per-section loading / error -------------------------------------------
    let loading = RwSignal::new([false; 4]);
    let set_loading = move |i: usize, v: bool| loading.update(|a| a[i] = v);
    let err_info = RwSignal::new(String::new());
    let err_rating = RwSignal::new(String::new());
    let err_subs = RwSignal::new(String::new());
    let err_blogs = RwSignal::new(String::new());

    let last_handles = RwSignal::new(Vec::<String>::new());

    let search = move || {
        let raw = handle_input.get_untracked();
        let handles: Vec<String> = raw
            .split([',', ';'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        if handles.is_empty() {
            err_info.set("Please enter at least one handle.".into());
            return;
        }
        err_info.set(String::new());
        err_rating.set(String::new());
        err_subs.set(String::new());
        err_blogs.set(String::new());
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

        set_loading(1, true);
        rating_changes.set(Vec::new());
        let h = primary.clone();
        spawn_local(async move {
            match api::user_rating(&h).await {
                Ok(c) => rating_changes.set(c),
                Err(e) => err_rating.set(e),
            }
            set_loading(1, false);
        });

        set_loading(2, true);
        let cnt: u32 = sub_count.get_untracked().parse().unwrap_or(50);
        let h = primary.clone();
        spawn_local(async move {
            match api::user_status(&h, 1, cnt).await {
                Ok(s) => submissions.set(s),
                Err(e) => err_subs.set(e),
            }
            set_loading(2, false);
        });

        set_loading(3, true);
        let h = primary;
        spawn_local(async move {
            match api::user_blog_entries(&h).await {
                Ok(b) => blogs.set(b),
                Err(e) => err_blogs.set(e),
            }
            set_loading(3, false);
        });
    };

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
            match api::user_status(&handles[0], 1, cnt).await {
                Ok(s) => submissions.set(s),
                Err(e) => err_subs.set(e),
            }
            set_loading(2, false);
        });
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
                let registered = format_date(u.registration_time_seconds);
                let online = format_time(u.last_online_time_seconds);
                let friends = u.friend_of_count;
                let photo = if u.title_photo.is_empty() {
                    "https://userpic.codeforces.org/no-title.jpg".to_string()
                } else {
                    u.title_photo.clone()
                };
                view! {
                    <div style="min-width:320px;flex:1;">
                        <Card>
                            <Flex vertical=true gap=FlexGap::Small>
                            <img src=photo style="width:100%;border-radius:6px;" alt="avatar"/>
                            <Text style="font-size:1.2em;font-weight:700;">
                                <a href=profile target="_blank" rel="noopener" style=format!("color:{rc};text-decoration:none;")>{handle.clone()}</a>
                            </Text>
                            <span style=format!("color:{rc};font-weight:600;")>{rank}</span>
                            <Caption1 style=format!("color:{mrc};")>{format!("Max: {max_rank}")}</Caption1>
                            <Divider/>
                            <p><b>"Rating: "</b><b style=format!("color:{rc};")>{rating}</b></p>
                            <p><b>"Max rating: "</b><b style=format!("color:{mrc};")>{max_rating}</b></p>
                            <p><b>"Contribution: "</b><b style=format!("color:{cc};")>{signed_delta(contribution)}</b></p>
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
                    <div style="max-height:420px;overflow-y:auto;border:1px solid #eee;border-radius:8px;padding:4px;">
                        <RatingChangeTable changes=changes show_handle=false/>
                    </div>
                </Flex>
            }
            .into_any()
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
                    <SectionHeader title="Rating history".into()/>
                    {rating_section()}

                    <SectionHeader title="Recent submissions".into()/>
                    <Flex gap=FlexGap::Small align=FlexAlign::Center>
                        <Text>"Show:"</Text>
                        <Select default_value="50" value=sub_count>
                            <option value="25">"25"</option>
                            <option value="50">"50"</option>
                            <option value="100">"100"</option>
                            <option value="250">"250"</option>
                        </Select>
                    </Flex>
                    <br/>
                    {move || -> AnyView {
                        if loading.get()[2] {
                            view! { <Loading label="Loading submissions".into()/> }.into_any()
                        } else if !err_subs.get().is_empty() {
                            view! { <ErrorBar message=err_subs/> }.into_any()
                        } else if submissions.with(|s| s.is_empty()) {
                            view! { <Empty text="No submissions found.".into()/> }.into_any()
                        } else {
                            view! { <SubmissionTable subs=submissions.get_untracked()/> }.into_any()
                        }
                    }}

                    <SectionHeader title="Blog entries".into()/>
                    {move || -> AnyView {
                        if loading.get()[3] {
                            view! { <Loading label="Loading blogs".into()/> }.into_any()
                        } else if !err_blogs.get().is_empty() {
                            view! { <ErrorBar message=err_blogs/> }.into_any()
                        } else if blogs.with(|b| b.is_empty()) {
                            view! { <Empty text="No blog entries found.".into()/> }.into_any()
                        } else {
                            view! { <BlogList blogs=blogs.get_untracked() on_open=open_blog/> }.into_any()
                        }
                    }}
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
                                            <div style="border-left:3px solid #ddd;padding-left:10px;margin-bottom:8px;">
                                                <p style="white-space:pre-wrap;">{text}</p>
                                                <Caption1>{who}" · "{when}</Caption1>
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
fn Stat(
    #[prop(into)] label: String,
    #[prop(into)] value: String,
    #[prop(optional)] color: Option<&'static str>,
) -> impl IntoView {
    let c = color.unwrap_or("#000");
    view! {
        <div style="padding:10px 16px;text-align:center;border:1px solid #eee;border-radius:6px;background:#fafafa;">
            <Caption1>{label.clone()}</Caption1>
            <p style=format!("font-size:1.3em;font-weight:700;color:{c};")>{value}</p>
        </div>
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
