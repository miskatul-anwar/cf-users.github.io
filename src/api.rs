//! Types and fetchers for the public Codeforces API.
//!
//! Covers every endpoint that does not require an API key:
//! blogEntry.comments, blogEntry.view, contest.hacks, contest.list,
//! contest.ratingChanges, contest.standings, contest.status,
//! problemset.problems, recentActions, user.blogEntries, user.info,
//! user.ratedList, user.rating, user.status.

use serde::Deserialize;

const API: &str = "https://codeforces.com/api";

// ---------------------------------------------------------------------------
// Envelope
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
struct ApiResponse<T> {
    status: String,
    #[serde(default)]
    comment: Option<String>,
    /// FAILED replies omit `result` entirely; a missing Option reads as None.
    result: Option<T>,
}

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

async fn get<T: for<'de> Deserialize<'de>>(
    method: &str,
    params: &[(&str, String)],
) -> Result<T, String> {
    let mut url = format!("{API}/{method}");
    if !params.is_empty() {
        url.push('?');
        let pairs: Vec<String> = params
            .iter()
            .map(|(k, v)| format!("{k}={}", percent_encode(v)))
            .collect();
        url.push_str(&pairs.join("&"));
    }
    let resp = reqwest::get(&url)
        .await
        .map_err(|e| format!("Network error: {e}"))?;
    let http = resp.status();
    // Application errors arrive as JSON (usually with HTTP 400), while a dead
    // endpoint serves an HTML error page; parse first so FAILED comments are
    // surfaced, and only fall back to the raw status if the body is not JSON.
    let body = resp
        .text()
        .await
        .map_err(|e| format!("Network error: {e}"))?;
    let api: ApiResponse<T> = serde_json::from_str(&body).map_err(|_| {
        format!("Codeforces returned an invalid response for {method} (HTTP {http})")
    })?;
    match api.status.as_str() {
        "OK" => api
            .result
            .ok_or_else(|| format!("Codeforces response for {method} is missing its result")),
        _ => Err(api.comment.unwrap_or_else(|| "Unknown error".into())),
    }
}

// ---------------------------------------------------------------------------
// Data models (field names mirror the JSON exactly)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct User {
    pub handle: String,
    pub email: Option<String>,
    #[serde(rename = "vkId")]
    pub vk_id: Option<String>,
    #[serde(rename = "openId")]
    pub open_id: Option<String>,
    #[serde(default)]
    pub first_name: Option<String>,
    #[serde(default)]
    pub last_name: Option<String>,
    pub country: Option<String>,
    pub city: Option<String>,
    pub organization: Option<String>,
    #[serde(default)]
    pub contribution: i32,
    pub rank: Option<String>,
    #[serde(default)]
    pub max_rank: Option<String>,
    #[serde(default)]
    pub rating: Option<i32>,
    #[serde(default)]
    pub max_rating: Option<i32>,
    #[serde(default)]
    pub last_online_time_seconds: i64,
    #[serde(default)]
    pub registration_time_seconds: i64,
    #[serde(default)]
    pub friend_of_count: i32,
    #[serde(default)]
    pub title_photo: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct RatingChange {
    #[serde(default)]
    pub contest_id: i64,
    #[serde(default)]
    pub contest_name: String,
    #[serde(default)]
    pub handle: String,
    #[serde(default)]
    pub rank: i32,
    #[serde(default)]
    pub rating_update_time_seconds: i64,
    #[serde(default)]
    pub old_rating: i32,
    #[serde(default)]
    pub new_rating: i32,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct Member {
    #[serde(default)]
    pub handle: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct Party {
    #[serde(default)]
    pub members: Vec<Member>,
    #[serde(default)]
    pub team_name: Option<String>,
    #[serde(default)]
    pub participant_type: String,
    #[serde(default)]
    pub ghost: bool,
}

impl Party {
    pub fn handles(&self) -> String {
        self.members
            .iter()
            .map(|m| m.handle.clone())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct Problem {
    #[serde(default)]
    pub contest_id: Option<i64>,
    #[serde(default)]
    pub problemset_name: Option<String>,
    #[serde(default)]
    pub index: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub problem_type: String,
    #[serde(default)]
    pub points: Option<f64>,
    #[serde(default)]
    pub rating: Option<i32>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub solved_count: i32,
}

impl Problem {
    /// Human readable code like "1234A".
    pub fn code(&self) -> String {
        self.contest_id
            .map(|id| format!("{id}{}", self.index))
            .unwrap_or_else(|| self.index.clone())
    }

    pub fn url(&self) -> String {
        match self.contest_id {
            Some(id) => format!("https://codeforces.com/contest/{id}/problem/{}", self.index),
            None => "https://codeforces.com/problemset".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct ProblemStatistics {
    #[serde(default)]
    pub solved_count: i32,
    #[serde(default)]
    pub attempted_count: i32,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct Submission {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub contest_id: Option<i64>,
    #[serde(default)]
    pub creation_time_seconds: i64,
    #[serde(default)]
    pub relative_time_seconds: i64,
    #[serde(default)]
    pub problem: Problem,
    #[serde(default)]
    pub programming_language: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub verdict: Option<String>,
    #[serde(default)]
    pub passed_test_count: i32,
    #[serde(default)]
    pub failed_test_count: i32,
    #[serde(default)]
    pub points: Option<f64>,
    #[serde(default)]
    pub author: Party,
}

impl Submission {
    pub fn url(&self) -> String {
        match self.contest_id {
            Some(id) => format!("https://codeforces.com/contest/{id}/submission/{}", self.id),
            None => format!("https://codeforces.com/submission/{}", self.id),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct Contest {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub contest_type: String,
    #[serde(default)]
    pub phase: String,
    #[serde(default)]
    pub frozen: bool,
    #[serde(default)]
    pub duration_seconds: i64,
    #[serde(default)]
    pub start_time_seconds: Option<i64>,
    #[serde(default)]
    pub relative_time_seconds: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct ProblemResult {
    #[serde(default)]
    pub points: f64,
    #[serde(default)]
    pub penalty: f64,
    #[serde(default)]
    pub rejected_attempt_count: i32,
    #[serde(default)]
    pub problem_result_type: String,
    #[serde(default)]
    pub best_submission_time_seconds: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct StandingsRow {
    #[serde(default)]
    pub position: i32,
    #[serde(default)]
    pub party: Party,
    #[serde(default)]
    pub points: f64,
    #[serde(default)]
    pub penalty: i64,
    #[serde(default)]
    pub successful_hack_count: i32,
    #[serde(default)]
    pub unsuccessful_hack_count: i32,
    #[serde(default)]
    pub problem_results: Vec<ProblemResult>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct ContestStandingsResult {
    #[serde(default)]
    pub contest: Contest,
    #[serde(default)]
    pub problems: Vec<Problem>,
    #[serde(default)]
    pub rows: Vec<StandingsRow>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct Hack {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub creation_time_seconds: i64,
    #[serde(default)]
    pub hacker: Party,
    #[serde(default)]
    pub defender: Party,
    #[serde(default)]
    pub verdict: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct BlogEntry {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub author_handle: String,
    #[serde(default)]
    pub creation_time_seconds: i64,
    #[serde(default)]
    pub is_viewed: bool,
    #[serde(default)]
    pub rating: Option<i32>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct Comment {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub creation_time_seconds: i64,
    #[serde(default)]
    pub commentator_handle: String,
    #[serde(default)]
    pub entry_id: i64,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub rating: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct RecentAction {
    #[serde(default)]
    pub time_seconds: i64,
    #[serde(default)]
    pub blog_entry: Option<BlogEntry>,
    #[serde(default)]
    pub comment: Option<Comment>,
}

/// `problemset.problems` returns two parallel arrays.
#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct ProblemSetResult {
    #[serde(default)]
    pub problems: Vec<Problem>,
    #[serde(default)]
    pub problem_statistics: Vec<ProblemStatistics>,
}

// ---------------------------------------------------------------------------
// Endpoints
// ---------------------------------------------------------------------------

pub async fn user_info(handles: &[&str]) -> Result<Vec<User>, String> {
    get(
        "user.info",
        &[
            ("handles", handles.join(";")),
            ("checkHistoricHandles", "false".into()),
        ],
    )
    .await
}

pub async fn user_rating(handle: &str) -> Result<Vec<RatingChange>, String> {
    get("user.rating", &[("handle", handle.into())]).await
}

pub async fn user_status(handle: &str, from: u32, count: u32) -> Result<Vec<Submission>, String> {
    get(
        "user.status",
        &[
            ("handle", handle.into()),
            ("from", from.to_string()),
            ("count", count.to_string()),
        ],
    )
    .await
}

pub async fn user_rated_list(active_only: bool) -> Result<Vec<User>, String> {
    get(
        "user.ratedList",
        &[(
            "activeOnly",
            if active_only { "true" } else { "false" }.to_string(),
        )],
    )
    .await
}

pub async fn user_blog_entries(handle: &str) -> Result<Vec<BlogEntry>, String> {
    get("user.blogEntries", &[("handle", handle.into())]).await
}

// NOTE: there is no `user.comments` in the public API — the method 404s with an
// HTML page. Per-user comment history is therefore not fetchable; blog-level
// comments remain available via `blogEntry.comments`.

pub async fn contest_list() -> Result<Vec<Contest>, String> {
    get("contest.list", &[("gym", "false".into())]).await
}

pub async fn contest_standings(
    contest_id: i64,
    from: u32,
    count: u32,
    handles: &[&str],
    show_unofficial: bool,
) -> Result<ContestStandingsResult, String> {
    let mut params = vec![
        ("contestId", contest_id.to_string()),
        ("from", from.to_string()),
        ("count", count.to_string()),
        ("showUnofficial", show_unofficial.to_string()),
    ];
    if !handles.is_empty() {
        params.push(("handles", handles.join(";")));
    }
    get("contest.standings", &params).await
}

pub async fn contest_rating_changes(contest_id: i64) -> Result<Vec<RatingChange>, String> {
    get(
        "contest.ratingChanges",
        &[("contestId", contest_id.to_string())],
    )
    .await
}

pub async fn contest_hacks(contest_id: i64) -> Result<Vec<Hack>, String> {
    get("contest.hacks", &[("contestId", contest_id.to_string())]).await
}

pub async fn contest_status(
    contest_id: i64,
    from: u32,
    count: u32,
    handle: Option<&str>,
) -> Result<Vec<Submission>, String> {
    let mut params = vec![
        ("contestId", contest_id.to_string()),
        ("from", from.to_string()),
        ("count", count.to_string()),
    ];
    if let Some(h) = handle.filter(|h| !h.is_empty()) {
        params.push(("handle", h.to_string()));
    }
    get("contest.status", &params).await
}

pub async fn problemset_problems(tags: &[&str]) -> Result<ProblemSetResult, String> {
    let params: Vec<(&str, String)> = vec![("tags", tags.join(";"))];
    let mut res: ProblemSetResult = get::<ProblemSetResult>("problemset.problems", &params).await?;
    // The API returns statistics aligned index-for-index with problems.
    for (p, st) in res.problems.iter_mut().zip(&res.problem_statistics) {
        p.solved_count = st.solved_count;
    }
    Ok(res)
}

pub async fn recent_actions(max_count: u32) -> Result<Vec<RecentAction>, String> {
    get("recentActions", &[("maxCount", max_count.to_string())]).await
}

pub async fn blog_entry_view(entry_id: i64) -> Result<BlogEntry, String> {
    get("blogEntry.view", &[("blogEntryId", entry_id.to_string())]).await
}

pub async fn blog_entry_comments(entry_id: i64) -> Result<Vec<Comment>, String> {
    get(
        "blogEntry.comments",
        &[("blogEntryId", entry_id.to_string())],
    )
    .await
}
