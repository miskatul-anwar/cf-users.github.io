//! Shared UI pieces: rating line chart (pure SVG), submission table,
//! rating-change table, error bar, loading spinner, section header,
//! activity heatmap, distribution bars, histograms, countdowns, exports,
//! multi-series comparison chart and the theme toggle.

use crate::api::{RatingChange, Submission};
use crate::storage;
use crate::store;
use crate::util::*;
use leptos::either::Either;
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
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

// ---------------------------------------------------------------------------
// Theme toggle
// ---------------------------------------------------------------------------

#[component]
pub fn ThemeToggle() -> impl IntoView {
    let dark = RwSignal::new(store::is_dark());
    view! {
        <Checkbox
            checked=dark
            label="Dark mode"
            on:click=move |_| {
                store::toggle_theme();
                dark.set(store::is_dark());
            }
        />
    }
}

// ---------------------------------------------------------------------------
// Live countdown ("2d 04:22:11") for upcoming contests
// ---------------------------------------------------------------------------

#[component]
pub fn Countdown(target_secs: i64) -> impl IntoView {
    let now = RwSignal::new(storage::now_secs());
    Effect::new(move |_| {
        let cb: leptos::wasm_bindgen::closure::Closure<dyn Fn()> = {
            let n = now;
            leptos::wasm_bindgen::closure::Closure::new(move || n.set(storage::now_secs()))
        };
        if let Ok(id) = window().set_interval_with_callback_and_timeout_and_arguments_0(
            cb.as_ref().unchecked_ref(),
            1000,
        ) {
            on_cleanup(move || {
                window().clear_interval_with_handle(id);
            });
        }
        cb.forget();
    });

    move || {
        let rem = target_secs - now.get();
        let text = if rem <= 0 {
            "Running / finished".to_string()
        } else {
            let d = rem / 86_400;
            let h = (rem % 86_400) / 3600;
            let m = (rem % 3600) / 60;
            let s = rem % 60;
            format!("{d}d {h:02}:{m:02}:{s:02}")
        };
        let live = rem <= 0;
        view! {
            <span style=format!(
                "font-variant-numeric:tabular-nums;font-weight:700;color:{};",
                if live { "#008000" } else { "#0078d4" }
            )>{text}</span>
        }
    }
}

// ---------------------------------------------------------------------------
// GitHub-style activity heatmap
// ---------------------------------------------------------------------------

#[component]
pub fn Heatmap(daily: Vec<(i64, u32)>) -> impl IntoView {
    let map: std::collections::HashMap<i64, u32> = daily.into_iter().collect();
    let today = day_of(storage::now_secs());
    let wd = weekday(today) as i64;
    // Grid starts on a Sunday so columns are whole weeks.
    let start = today - wd - 52 * 7;
    let cell = 12.0_f64;
    let pitch = 14.5_f64;
    let top = 18.0_f64;
    let cols = 53.0_f64;
    let w = cols * pitch;
    let h = top + 7.0 * pitch + 4.0;

    let total: i64 = ((start.max(0))..=today)
        .filter_map(|d| map.get(&d).copied())
        .map(|c| c as i64)
        .sum();

    let level_color = |c: u32| match c {
        0 => "rgba(128,128,128,0.14)".to_string(),
        1..=2 => "rgba(46,160,67,0.35)".to_string(),
        3..=5 => "rgba(46,160,67,0.6)".to_string(),
        6..=9 => "rgba(46,160,67,0.85)".to_string(),
        _ => "rgba(46,160,67,1)".to_string(),
    };

    struct Cell {
        x: f64,
        y: f64,
        day: i64,
        count: u32,
    }
    let mut cells: Vec<Cell> = Vec::new();
    let mut months: Vec<(f64, String)> = Vec::new();
    let mut last_month: Option<u32> = None;
    for col in 0..53i64 {
        for row in 0..7i64 {
            let day = start + col * 7 + row;
            if day < start || day > today {
                continue;
            }
            cells.push(Cell {
                x: col as f64 * pitch,
                y: top + row as f64 * pitch,
                day,
                count: map.get(&day).copied().unwrap_or(0),
            });
        }
        let first_day = (start + col * 7).min(today);
        let (_, m, _) = civil_from_days(first_day);
        if last_month != Some(m) {
            last_month = Some(m);
            const MONTHS: [&str; 12] = [
                "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
            ];
            months.push((col as f64 * pitch, MONTHS[(m - 1) as usize].into()));
        }
    }

    view! {
        <div style="overflow-x:auto;">
            <svg viewBox=format!("0 0 {w} {h}") style="width:100%;min-width:720px;height:auto;">
                {months
                    .into_iter()
                    .map(|(x, m)| {
                        view! {
                            <text x=x y=11.0 font-size="10" fill="#888">{m}</text>
                        }
                    })
                    .collect_view()}
                {cells
                    .into_iter()
                    .map(|c| {
                        let fill = level_color(c.count);
                        let tip = if c.count == 0 {
                            format!("No solves \u{2014} {}", day_label(c.day))
                        } else {
                            format!(
                                "{} solve{} \u{2014} {}",
                                c.count,
                                if c.count > 1 { "s" } else { "" },
                                day_label(c.day)
                            )
                        };
                        view! {
                            <rect
                                x=c.x
                                y=c.y
                                width=cell
                                height=cell
                                rx="3"
                                fill=fill
                            >
                                <title>{tip}</title>
                            </rect>
                        }
                    })
                    .collect_view()}
            </svg>
            <Caption1 style="color:#888;">
                {format!("{total} solves in the last year \u{00b7} ")}"Less "
                <span style="display:inline-block;width:10px;height:10px;border-radius:2px;background:rgba(128,128,128,0.14);vertical-align:middle;"></span>
                <span style="display:inline-block;width:10px;height:10px;border-radius:2px;background:rgba(46,160,67,0.35);vertical-align:middle;"></span>
                <span style="display:inline-block;width:10px;height:10px;border-radius:2px;background:rgba(46,160,67,0.6);vertical-align:middle;"></span>
                <span style="display:inline-block;width:10px;height:10px;border-radius:2px;background:rgba(46,160,67,1);vertical-align:middle;"></span>
                " More"
            </Caption1>
        </div>
    }
}

/// Horizontal distribution bars.
#[component]
pub fn NamedCountBar(items: Vec<NamedCount>, color: &'static str) -> impl IntoView {
    let max = items.iter().map(|i| i.count).max().unwrap_or(1).max(1);
    view! {
        <div style="display:flex;flex-direction:column;gap:6px;">
            {items
                .into_iter()
                .map(|it| {
                    let w = (it.count as f64 / max as f64 * 100.0).clamp(1.5, 100.0);
                    let label = it.label.clone();
                    let tip = format!("{} ({})", label, thousands(it.count));
                    view! {
                        <div style="display:flex;align-items:center;gap:8px;">
                            <div
                                title=tip
                                style="flex:1;display:flex;align-items:center;gap:8px;min-width:0;"
                            >
                                <Caption1 style="width:150px;text-align:right;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;flex-shrink:0;">
                                    {label}
                                </Caption1>
                                <div style=format!(
                                    "height:16px;width:{w:.1}%;min-width:4px;background:{color};border-radius:3px;opacity:0.75;",
                                )></div>
                                <Caption1 style="flex-shrink:0;">{thousands(it.count)}</Caption1>
                            </div>
                        </div>
                    }
                })
                .collect_view()}
        </div>
    }
}

#[derive(Clone, Debug)]
pub struct NamedCount {
    pub label: String,
    pub count: i64,
}

/// Vertical histogram over labelled buckets.
#[component]
pub fn Histogram(bars: Vec<(String, i64, &'static str)>) -> impl IntoView {
    let bw = 34.0_f64;
    let gap = 8.0_f64;
    let pad_t = 18.0_f64;
    let pad_b = 26.0_f64;
    let max = bars.iter().map(|b| b.1).max().unwrap_or(1).max(1);
    let chart_h = 120.0_f64;
    let w = bars.len() as f64 * (bw + gap);
    let h = pad_t + chart_h + pad_b;

    view! {
        <div style="overflow-x:auto;">
            <svg viewBox=format!("0 0 {w} {h}") style="width:100%;height:auto;">
                {bars
                    .into_iter()
                    .enumerate()
                    .filter(|(_, (_, c, _))| *c > 0)
                    .map(|(i, (label, count, color))| {
                        let bh = count as f64 / max as f64 * chart_h;
                        let x = i as f64 * (bw + gap);
                        let y = pad_t + chart_h - bh;
                        let cx = x + bw / 2.0;
                        view! {
                            <g>
                                <title>{format!("{label}: {} solved", thousands(count))}</title>
                                <rect x=x y=y width=bw height=bh rx="3" fill=color opacity="0.8"></rect>
                                <text x=cx y=y - 4.0 font-size="9" fill="#999" text-anchor="middle">
                                    {thousands(count)}
                                </text>
                                <text x=cx y=h - 8.0 font-size="9" fill="#888" text-anchor="middle">{label}</text>
                            </g>
                        }
                    })
                    .collect_view()}
            </svg>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Multi-series rating chart (user comparison)
// ---------------------------------------------------------------------------

pub struct SeriesChanges {
    pub name: String,
    pub color: &'static str,
    pub changes: Vec<RatingChange>,
}

#[derive(Clone)]
struct Pt {
    t: f64,
    r: f64,
}

#[component]
pub fn MultiRatingChart(series: Vec<SeriesChanges>) -> impl IntoView {
    let all_empty = series.iter().all(|s| s.changes.len() < 2);
    if all_empty {
        return Either::Left(view! {
            <Empty text="Not enough rated contests to draw a chart.".into()/>
        });
    }

    let w = 900.0_f64;
    let h = 280.0_f64;
    let pad_l = 55.0_f64;
    let pad_r = 90.0_f64;
    let pad_t = 16.0_f64;
    let pad_b = 30.0_f64;

    let pts_per_series: Vec<Vec<Pt>> = series
        .iter()
        .map(|s| {
            s.changes
                .iter()
                .map(|c| Pt {
                    t: c.rating_update_time_seconds as f64,
                    r: c.new_rating as f64,
                })
                .collect()
        })
        .collect();

    let mut lo = f64::MAX;
    let mut hi = f64::MIN;
    let mut t_min = f64::MAX;
    let mut t_max = f64::MIN;
    for pts in &pts_per_series {
        for p in pts {
            lo = lo.min(p.r - 40.0);
            hi = hi.max(p.r + 40.0);
            t_min = t_min.min(p.t);
            t_max = t_max.max(p.t);
        }
    }
    if t_min == f64::MAX {
        t_min = 0.0;
        t_max = 1.0;
    }
    if hi <= lo {
        hi = lo + 400.0;
    }
    if t_max - t_min < 1.0 {
        t_max = t_min + 1.0;
    }

    let span = t_max - t_min;
    let x_of = |t: f64| pad_l + (t - t_min) / span * (w - pad_l - pad_r);
    let y_of = |r: f64| pad_t + (hi - r) / (hi - lo) * (h - pad_t - pad_b);

    let step = match ((hi - lo) / 400.0) as i32 {
        v if v <= 3 => 100,
        v if v <= 7 => 200,
        v if v <= 15 => 300,
        _ => 500,
    };
    let lo_i = (lo as i32 / step) * step;
    let gridlines: Vec<(i32, f64)> = (lo_i..=(hi as i32))
        .step_by(step as usize)
        .map(|v| (v, y_of(v as f64)))
        .collect();

    let x_ticks: Vec<(f64, String)> = (0..=4)
        .map(|k| {
            let t = t_min + span * k as f64 / 4.0;
            (x_of(t), format_date(t as i64))
        })
        .collect();

    let lines: Vec<(String, &'static str, Vec<Pt>)> = series
        .iter()
        .zip(&pts_per_series)
        .map(|(s, pts)| (s.name.clone(), s.color, pts.clone()))
        .collect();

    let legend: Vec<_> = series
        .iter()
        .map(|s| (s.name.clone(), s.color))
        .map(|(name, color)| {
            view! {
                <Caption1>
                    <span style=format!(
                        "display:inline-block;width:10px;height:10px;border-radius:50%;background:{color};margin-right:5px;",
                    )></span>
                    {name}
                </Caption1>
            }
        })
        .collect_view();

    Either::Right(view! {
        <div>
            <Flex gap=FlexGap::Medium style="margin-bottom:6px;flex-wrap:wrap;">
                {legend}
            </Flex>
            <div style="overflow-x:auto;">
                <svg viewBox=format!("0 0 {w} {h}") style="width:100%;min-width:600px;height:auto;">
                    {gridlines
                        .into_iter()
                        .map(|(v, y)| {
                            view! {
                                <line
                                    x1=pad_l
                                    y1=y
                                    x2=w - pad_r
                                    y2=y
                                    stroke=rating_color(v)
                                    stroke-width="0.5"
                                    stroke-dasharray="3,3"
                                    opacity="0.45"
                                ></line>
                                <text x=pad_l - 6.0 y=y + 4.0 font-size="11" fill="#666" text-anchor="end">{v}</text>
                            }
                        })
                        .collect_view()}
                    {x_ticks
                        .into_iter()
                        .map(|(x, lbl)| {
                            view! {
                                <text x=x y=h - 8.0 font-size="10" fill="#999" text-anchor="middle">{lbl}</text>
                            }
                        })
                        .collect_view()}
                    {lines
                        .into_iter()
                        .map(|(name, color, pts)| {
                            let polyline = pts
                                .iter()
                                .map(|p| format!("{:.1},{:.1}", x_of(p.t), y_of(p.r)))
                                .collect::<Vec<_>>()
                                .join(" ");
                            let last = pts.last().cloned().unwrap_or(Pt { t: 0.0, r: 0.0 });
                            let lx = x_of(last.t);
                            let ly = y_of(last.r);
                            view! {
                                <g>
                                    <polyline
                                        points=polyline
                                        fill="none"
                                        stroke=color
                                        stroke-width="2.5"
                                        stroke-linejoin="round"
                                    ></polyline>
                                    {pts.iter().map(|p| {
                                        let px = x_of(p.t);
                                        let py = y_of(p.r);
                                        view! {
                                            <circle cx=px cy=py r="2.5" fill=color opacity="0.7">
                                                <title>{format!("{name}: {} \u{2014} {}", p.r as i32, format_date(p.t as i64))}</title>
                                            </circle>
                                        }
                                    }).collect_view()}
                                    <circle cx=lx cy=ly r="5" fill=color stroke="#fff" stroke-width="2"></circle>
                                    <text x=lx + 8.0 y=ly + 4.0 font-size="12" font-weight="600" fill=color>
                                        {name.clone()}
                                    </text>
                                </g>
                            }
                        })
                        .collect_view()}
                </svg>
            </div>
        </div>
    })
}

// ---------------------------------------------------------------------------
// Export / share helpers
// ---------------------------------------------------------------------------

#[component]
pub fn DownloadButton(
    #[prop(into)] filename: String,
    #[prop(into)] content: String,
    children: Children,
) -> impl IntoView {
    view! {
        <Button
            appearance=ButtonAppearance::Subtle
            size=ButtonSize::Small
            on:click=move |_| storage::download(&filename.clone(), &content.clone())
        >
            {children()}
        </Button>
    }
}

#[component]
pub fn CopyLinkButton(#[prop(into)] text: String) -> impl IntoView {
    let copied = RwSignal::new(false);
    view! {
        <Button
            appearance=ButtonAppearance::Subtle
            size=ButtonSize::Small
            on:click=move |_| {
                storage::copy_text(&text.clone());
                copied.set(true);
                reset_later(1400, move || copied.set(false));
            }
        >
            {move || if copied.get() { "Copied!" } else { "Copy link" }}
        </Button>
    }
}

fn reset_later(ms: i32, f: impl Fn() + 'static) {
    use leptos::wasm_bindgen::{JsCast, closure::Closure};
    let cb = Closure::wrap(Box::new(f) as Box<dyn Fn()>);
    let _ = window()
        .set_timeout_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), ms);
    cb.forget();
}

// ---------------------------------------------------------------------------
// Stat tiles & pills
// ---------------------------------------------------------------------------

/// Small stat tile used across dashboards.
#[component]
pub fn Stat(
    #[prop(into)] label: String,
    #[prop(into)] value: String,
    #[prop(optional)] color: Option<&'static str>,
) -> impl IntoView {
    let c = color.unwrap_or("#000");
    view! {
        <div style="padding:10px 16px;text-align:center;border:1px solid rgba(128,128,128,0.25);border-radius:8px;background:rgba(128,128,128,0.06);">
            <Caption1>{label.clone()}</Caption1>
            <p style=format!("font-size:1.3em;font-weight:700;color:{c};margin:2px 0 0;")>{value}</p>
        </div>
    }
}

/// Colored rating chip.
#[component]
pub fn RatingPill(rating: i32) -> impl IntoView {
    let c = rating_color(rating);
    view! {
        <span style=format!(
            "background:{c};color:#fff;padding:1px 8px;border-radius:999px;font-size:0.78rem;font-weight:700;"
        )>{rating}</span>
    }
}
