//! Shared UI pieces: rating line chart (pure SVG), submission table,
//! rating-change table, error bar, loading spinner, section header.

use crate::api::{RatingChange, Submission};
use crate::util::*;
use leptos::either::Either;
use leptos::prelude::*;
use thaw::*;

// ---------------------------------------------------------------------------
// Small building blocks
// ---------------------------------------------------------------------------

#[component]
pub fn SectionHeader(title: String) -> impl IntoView {
    view! {
        <div style="display:flex;align-items:center;gap:8px;margin:18px 0 6px;">
            <Text style="font-size:1.15em;font-weight:600;">{title}</Text>
        </div>
    }
}

#[component]
pub fn ErrorBar(#[prop(into)] message: Signal<String>) -> impl IntoView {
    move || {
        let msg = message.get();
        (!msg.is_empty()).then(|| {
            view! {
                <MessageBar intent=MessageBarIntent::Error>
                    <MessageBarBody>
                        <MessageBarTitle>"API error"</MessageBarTitle>
                        {msg}
                    </MessageBarBody>
                </MessageBar>
            }
        })
    }
}

#[component]
pub fn Loading(label: String) -> impl IntoView {
    view! {
        <div style="display:flex;justify-content:center;padding:24px;">
            <Spinner label />
        </div>
    }
}

#[component]
pub fn Empty(text: String) -> impl IntoView {
    view! { <p style="color:#888;text-align:center;">{text}</p> }
}

/// Handle link to the user's profile on Codeforces.
#[component]
pub fn HandleLink(handle: String) -> impl IntoView {
    let href = format!("https://codeforces.com/profile/{}", handle.clone());
    view! {
        <a href=href target="_blank" rel="noopener">
            {handle}
        </a>
    }
}

// ---------------------------------------------------------------------------
// Rating history line chart (pure SVG)
// ---------------------------------------------------------------------------

#[component]
pub fn RatingChart(changes: Vec<RatingChange>) -> impl IntoView {
    if changes.len() < 2 {
        return Either::Left(view! {
            <Empty text="Not enough rated contests yet to draw a chart.".into()/>
        });
    }

    let w = 900.0_f64;
    let h = 260.0_f64;
    let pad_l = 55.0_f64;
    let pad_r = 20.0_f64;
    let pad_t = 14.0_f64;
    let pad_b = 30.0_f64;

    let ratings: Vec<i32> = changes.iter().map(|c| c.new_rating).collect();
    let mut lo = *ratings.iter().min().unwrap();
    let mut hi = *ratings.iter().max().unwrap();
    for c in &changes {
        lo = lo.min(c.old_rating);
        hi = hi.max(c.new_rating);
    }
    lo = (lo / 100 - 1).max(0) * 100;
    hi = (hi / 100 + 2) * 100;
    if hi - lo < 400 {
        hi = lo + 400;
    }

    let n = changes.len() as f64;
    let x_of = move |i: usize| pad_l + (i as f64) * (w - pad_l - pad_r) / (n - 1.0).max(1.0);
    let y_of = move |r: i32| pad_t + (hi - r) as f64 / (hi - lo) as f64 * (h - pad_t - pad_b);

    let pts: Vec<String> = ratings
        .iter()
        .enumerate()
        .map(|(i, r)| format!("{:.1},{:.1}", x_of(i), y_of(*r)))
        .collect();
    let polyline = pts.join(" ");
    let first_x = x_of(0);
    let last_i = ratings.len() - 1;
    let last_x = x_of(last_i);
    let last_y = y_of(ratings[last_i]);
    let baseline = h - pad_b;
    let area = format!("M{first_x:.1},{baseline:.1} L{polyline} L{last_x:.1},{baseline:.1} Z");

    let final_color = rating_color(ratings[last_i]);

    // Horizontal gridlines every `step` rating points.
    let step = match (hi - lo) / 400 {
        0..=3 => 100,
        4..=7 => 200,
        8..=15 => 300,
        _ => 500,
    };
    let gridlines: Vec<(i32, f64, &'static str)> = (((lo / step + 1) * step)..hi)
        .step_by(step as usize)
        .map(|v| {
            (
                v,
                y_of(v),
                if (1200..=3500).contains(&v) {
                    rating_color(v)
                } else {
                    "#d0d0d0"
                },
            )
        })
        .collect();

    let dots: Vec<(f64, f64, i32, String)> = ratings
        .iter()
        .enumerate()
        .map(|(i, r)| {
            (
                x_of(i),
                y_of(*r),
                *r,
                format_date(changes[i].rating_update_time_seconds),
            )
        })
        .collect();

    let title_left = format!(
        "{} ({})",
        truncate(&changes[0].contest_name, 34),
        format_date(changes[0].rating_update_time_seconds)
    );
    let title_right = format!(
        "{} ({})",
        truncate(&changes[last_i].contest_name, 34),
        format_date(changes[last_i].rating_update_time_seconds)
    );

    Either::Right(view! {
        <div style="overflow-x:auto;">
            <svg viewBox=format!("0 0 {w} {h}") style="width:100%;min-width:600px;height:auto;background:#fafafa;border-radius:8px;">
                {gridlines.into_iter().map(|(v, y, c)| view! {
                    <line x1=pad_l y1=y x2=w-pad_r y2=y stroke=c stroke-width="0.5" stroke-dasharray="3,3" opacity="0.5"/>
                    <text x=pad_l-6.0 y=y+4.0 font-size="11" fill="#666" text-anchor="end">{v}</text>
                }).collect_view()}
                <path d=area fill=final_color opacity="0.08"/>
                <polyline points=polyline fill="none" stroke=final_color stroke-width="2.5" stroke-linejoin="round"/>
                {dots.into_iter().map(|(x, y, r, d)| view! {
                    <circle cx=x cy=y r="3" fill="#fff" stroke=rating_color(r) stroke-width="2">
                        <title>{format!("{r} \u{2014} {d}")}</title>
                    </circle>
                }).collect_view()}
                <text x=pad_l y=h-6.0 font-size="10" fill="#999" text-anchor="start">{title_left}</text>
                <text x=w-pad_r y=h-6.0 font-size="10" fill="#999" text-anchor="end">{title_right}</text>
                <circle cx=last_x cy=last_y r="5" fill=final_color stroke="#fff" stroke-width="2"/>
            </svg>
        </div>
    })
}

// ---------------------------------------------------------------------------
// Tables
// ---------------------------------------------------------------------------

#[component]
pub fn SubmissionTable(subs: Vec<Submission>) -> impl IntoView {
    view! {
        <Table>
            <TableHeader>
                <TableRow>
                    <TableHeaderCell>"When"</TableHeaderCell>
                    <TableHeaderCell>"Problem"</TableHeaderCell>
                    <TableHeaderCell>"Language"</TableHeaderCell>
                    <TableHeaderCell>"Verdict"</TableHeaderCell>
                    <TableHeaderCell>"Tests"</TableHeaderCell>
                    <TableHeaderCell>"Points"</TableHeaderCell>
                </TableRow>
            </TableHeader>
            <TableBody>
                {subs
                    .into_iter()
                    .map(|s| {
                        let when = format_time(s.creation_time_seconds);
                        let lang = s.programming_language.clone();
                        let verdict = s.verdict.clone().unwrap_or_else(|| "TESTING".into());
                        let vt = verdict_text(&verdict);
                        let vc = verdict_color(&verdict);
                        let tests = s.passed_test_count;
                        let pts = s.points.unwrap_or(0.0);
                        let p = s.problem.code();
                        let pname = format!("{} {}", p, s.problem.name);
                        let plink = s.url();
                        let prating = s.problem.rating;
                        view! {
                            <TableRow>
                                <TableCell><TableCellLayout>{when}</TableCellLayout></TableCell>
                                <TableCell>
                                    <TableCellLayout>
                                        <a href=plink target="_blank" rel="noopener">{pname}</a>
                                        {prating.map(|r| view! {
                                            <span style=format!("margin-left:6px;color:{};", rating_color(r))>{r}</span>
                                        })}
                                    </TableCellLayout>
                                </TableCell>
                                <TableCell><TableCellLayout truncate=true>{lang}</TableCellLayout></TableCell>
                                <TableCell><span style=format!("font-weight:600;color:{vc};")>{vt}</span></TableCell>
                                <TableCell>{tests}</TableCell>
                                <TableCell>{format!("{pts:.1}")}</TableCell>
                            </TableRow>
                        }
                    })
                    .collect_view()}
            </TableBody>
        </Table>
    }
}

#[component]
pub fn RatingChangeTable(changes: Vec<RatingChange>, show_handle: bool) -> impl IntoView {
    let total = changes.len();
    view! {
        <Table>
            <TableHeader>
                <TableRow>
                    <TableHeaderCell>"#"</TableHeaderCell>
                    {if show_handle {
                        Some(view! { <TableHeaderCell>"Handle"</TableHeaderCell> })
                    } else {
                        None
                    }}
                    <TableHeaderCell>"Date"</TableHeaderCell>
                    <TableHeaderCell>"Contest"</TableHeaderCell>
                    <TableHeaderCell>"Rank"</TableHeaderCell>
                    <TableHeaderCell>"Old"</TableHeaderCell>
                    <TableHeaderCell>"New"</TableHeaderCell>
                    <TableHeaderCell>"Delta"</TableHeaderCell>
                </TableRow>
            </TableHeader>
            <TableBody>
                {changes
                    .into_iter()
                    .rev()
                    .enumerate()
                    .map(|(i, c)| {
                        let n = total - i;
                        let date = format_date(c.rating_update_time_seconds);
                        let contest = truncate(&c.contest_name, 48);
                        let clink = format!("https://codeforces.com/contest/{}", c.contest_id);
                        let handle = c.handle.clone();
                        let old = c.old_rating;
                        let new = c.new_rating;
                        let delta = new - old;
                        let dc = if delta >= 0 { "#008000" } else { "#ff0000" };
                        let delta_s = signed_delta(delta);
                        let nc = rating_color(new);
                        view! {
                            <TableRow>
                                <TableCell>{n}</TableCell>
                                {if show_handle {
                                    Some(view! { <TableCell><HandleLink handle/></TableCell> })
                                } else {
                                    None
                                }}
                                <TableCell>{date}</TableCell>
                                <TableCell><a href=clink target="_blank" rel="noopener">{contest}</a></TableCell>
                                <TableCell>{c.rank}</TableCell>
                                <TableCell>{old}</TableCell>
                                <TableCell><b style=format!("color:{nc};")>{new}</b></TableCell>
                                <TableCell><b style=format!("color:{dc};")>{delta_s}</b></TableCell>
                            </TableRow>
                        }
                    })
                    .collect_view()}
            </TableBody>
        </Table>
    }
}
