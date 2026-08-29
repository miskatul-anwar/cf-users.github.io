//! Types and fetchers for the public Codeforces API.
//!
//! Covers every endpoint that does not require an API key:
//! blogEntry.comments, blogEntry.view, contest.hacks, contest.list,
//! contest.ratingChanges, contest.standings, contest.status,
//! problemset.problems, recentActions, user.blogEntries, user.info,
//! user.ratedList, user.rating, user.status.

use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub handle: String,
    pub email: Option<String>,
    pub vk_id: Option<String>,
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
    pub avatar: String,
    #[serde(default)]
    pub title_photo: String,
}

impl User {
    pub fn photo_url(&self) -> String {
        let raw = if !self.title_photo.is_empty() {
            &self.title_photo
        } else if !self.avatar.is_empty() {
            &self.avatar
        } else {
            "https://userpic.codeforces.org/no-title.jpg"
        };
        if raw.starts_with("//") {
            format!("https:{raw}")
        } else {
            raw.to_string()
        }
    }

    pub fn full_name(&self) -> Option<String> {
        match (&self.first_name, &self.last_name) {
            (Some(f), Some(l)) if !f.is_empty() && !l.is_empty() => Some(format!("{f} {l}")),
            (Some(f), _) if !f.is_empty() => Some(f.clone()),
            (_, Some(l)) if !l.is_empty() => Some(l.clone()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Member {
    #[serde(default)]
    pub handle: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    #[serde(default)]
    pub contest_id: Option<i64>,
    #[serde(default)]
    pub problemset_name: Option<String>,
    #[serde(default)]
    pub index: String,
    #[serde(default)]
    pub name: String,
    #[serde(rename = "type", default)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProblemStatistics {
    #[serde(default)]
    pub contest_id: Option<i64>,
    #[serde(default)]
    pub index: String,
    #[serde(default)]
    pub solved_count: i32,
    #[serde(default)]
    pub attempted_count: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Contest {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(rename = "type", default)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProblemResult {
    #[serde(default)]
    pub points: f64,
    #[serde(default)]
    pub penalty: f64,
    #[serde(default)]
    pub rejected_attempt_count: i32,
    #[serde(rename = "type", default)]
    pub problem_result_type: String,
    #[serde(default)]
    pub best_submission_time_seconds: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StandingsRow {
    #[serde(alias = "rank", default)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ContestStandingsResult {
    #[serde(default)]
    pub contest: Contest,
    #[serde(default)]
    pub problems: Vec<Problem>,
    #[serde(default)]
    pub rows: Vec<StandingsRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RecentAction {
    #[serde(default)]
    pub time_seconds: i64,
    #[serde(default)]
    pub blog_entry: Option<BlogEntry>,
    #[serde(default)]
    pub comment: Option<Comment>,
}

/// `problemset.problems` returns two parallel arrays.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
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

// ---------------------------------------------------------------------------
// Cached fetchers (localStorage TTL cache so heavy endpoints load instantly
// on repeat visits; quota errors are swallowed silently by ls_set).
// ---------------------------------------------------------------------------

use crate::storage::{ls_get, ls_set, now_secs};

fn cache_get(key: &str, ttl_secs: i64) -> Option<String> {
    let raw = ls_get(&format!("cfx:{key}"))?;
    let (ts, json) = raw.split_once('\u{1}')?;
    let ts: i64 = ts.parse().ok()?;
    (now_secs() - ts < ttl_secs).then(|| json.to_string())
}

async fn get_cached<T: serde::de::DeserializeOwned + serde::Serialize + Clone>(
    key: &str,
    ttl_secs: i64,
    method: &str,
    params: &[(&str, String)],
) -> Result<T, String> {
    if let Some(json) = cache_get(key, ttl_secs)
        && let Ok(v) = serde_json::from_str::<T>(&json)
    {
        return Ok(v);
    }
    let v = get(method, params).await?;
    if let Ok(json) = serde_json::to_string(&v) {
        ls_set(&format!("cfx:{key}"), &format!("{}\u{1}{json}", now_secs()));
    }
    Ok(v)
}

/// Rating history cached for 10 minutes.
pub async fn user_rating_cached(handle: &str) -> Result<Vec<RatingChange>, String> {
    get_cached(
        &format!("rating:{handle}"),
        600,
        "user.rating",
        &[("handle", handle.into())],
    )
    .await
}

/// Submission history cached for 10 minutes (analytics pulls up to a few
/// thousand rows, well worth persisting between visits).
pub async fn user_status_cached(handle: &str, count: u32) -> Result<Vec<Submission>, String> {
    get_cached(
        &format!("status:{handle}:{count}"),
        600,
        "user.status",
        &[
            ("handle", handle.into()),
            ("from", "1".into()),
            ("count", count.to_string()),
        ],
    )
    .await
}

/// Contest list cached for 6 hours (~1.5 MB JSON).
pub async fn contest_list_cached() -> Result<Vec<Contest>, String> {
    let mut c: Vec<Contest> = get_cached(
        "contests",
        21_600,
        "contest.list",
        &[("gym", "false".into())],
    )
    .await?;
    c.sort_by_key(|c| c.start_time_seconds.unwrap_or(0));
    Ok(c)
}

/// Compact problem row for the localStorage cache (~1 MB instead of ~8 MB).
#[derive(serde::Serialize, serde::Deserialize)]
struct CProb(i64, String, String, i32, i32, Vec<String>);

fn problemset_cache_key(tags: &[&str]) -> String {
    if tags.is_empty() {
        "pset:all".into()
    } else {
        format!("pset:{}", tags.join("+"))
    }
}

/// `problemset.problems` cached for 24 hours in a compact representation.
pub async fn problemset_problems_cached() -> Result<ProblemSetResult, String> {
    let key = problemset_cache_key(&[]);
    if let Some(json) = cache_get(&key, 86_400)
        && let Ok(list) = serde_json::from_str::<Vec<CProb>>(&json)
    {
        let problems = list
            .into_iter()
            .map(|CProb(cid, index, name, rating, solved, tags)| Problem {
                contest_id: (cid > 0).then_some(cid),
                index,
                name,
                rating: (rating > 0).then_some(rating),
                solved_count: solved,
                tags,
                ..Default::default()
            })
            .collect();
        return Ok(ProblemSetResult {
            problems,
            problem_statistics: Vec::new(),
        });
    }
    let res = get::<ProblemSetResult>("problemset.problems", &[("tags", String::new())]).await?;
    // The API returns statistics aligned index-for-index with problems.
    let mut res = res;
    for (p, st) in res.problems.iter_mut().zip(&res.problem_statistics) {
        p.solved_count = st.solved_count;
    }
    let compact: Vec<CProb> = res
        .problems
        .iter()
        .map(|p| {
            CProb(
                p.contest_id.unwrap_or(-1),
                p.index.clone(),
                p.name.clone(),
                p.rating.unwrap_or(0),
                p.solved_count,
                p.tags.clone(),
            )
        })
        .collect();
    if let Ok(json) = serde_json::to_string(&compact) {
        ls_set(&format!("cfx:{key}"), &format!("{}\u{1}{json}", now_secs()));
    }
    Ok(res)
}

/// Recent actions cached for 3 minutes to keep the feed snappy on tab switches.
pub async fn recent_actions_cached(max_count: u32) -> Result<Vec<RecentAction>, String> {
    get_cached(
        &format!("recent:{max_count}"),
        180,
        "recentActions",
        &[("maxCount", max_count.to_string())],
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_deserialization() {
        let json = r#"{
            "handle": "tourist",
            "email": "tourist@example.com",
            "firstName": "Gennady",
            "lastName": "Korotkevich",
            "country": "Belarus",
            "city": "Gomel",
            "organization": "ITMO University",
            "contribution": 109,
            "rank": "legendary grandmaster",
            "rating": 3528,
            "maxRank": "tourist",
            "maxRating": 4009,
            "lastOnlineTimeSeconds": 1788012225,
            "registrationTimeSeconds": 1265987288,
            "friendOfCount": 90514,
            "avatar": "https://userpic.codeforces.org/422/avatar/2b5dbe87f0d859a2.jpg",
            "titlePhoto": "https://userpic.codeforces.org/422/title/50a270ed4a722867.jpg"
        }"#;

        let user: User = serde_json::from_str(json).expect("deserialize user");
        assert_eq!(user.handle, "tourist");
        assert_eq!(user.first_name.as_deref(), Some("Gennady"));
        assert_eq!(user.last_name.as_deref(), Some("Korotkevich"));
        assert_eq!(user.full_name().as_deref(), Some("Gennady Korotkevich"));
        assert_eq!(user.rating, Some(3528));
        assert_eq!(user.max_rating, Some(4009));
        assert_eq!(user.max_rank.as_deref(), Some("tourist"));
        assert_eq!(user.last_online_time_seconds, 1788012225);
        assert_eq!(user.registration_time_seconds, 1265987288);
        assert_eq!(user.friend_of_count, 90514);
        assert_eq!(
            user.photo_url(),
            "https://userpic.codeforces.org/422/title/50a270ed4a722867.jpg"
        );
    }

    #[test]
    fn test_rating_change_deserialization() {
        let json = r#"{
            "contestId": 2,
            "contestName": "Codeforces Beta Round 2",
            "handle": "tourist",
            "rank": 14,
            "ratingUpdateTimeSeconds": 1267124400,
            "oldRating": 0,
            "newRating": 1602
        }"#;

        let rc: RatingChange = serde_json::from_str(json).expect("deserialize rating change");
        assert_eq!(rc.contest_id, 2);
        assert_eq!(rc.contest_name, "Codeforces Beta Round 2");
        assert_eq!(rc.handle, "tourist");
        assert_eq!(rc.rank, 14);
        assert_eq!(rc.rating_update_time_seconds, 1267124400);
        assert_eq!(rc.old_rating, 0);
        assert_eq!(rc.new_rating, 1602);
    }

    #[test]
    fn test_problem_and_submission_deserialization() {
        let json = r#"{
            "id": 383013765,
            "contestId": 2245,
            "creationTimeSeconds": 1784221884,
            "relativeTimeSeconds": 8783,
            "problem": {
                "contestId": 2245,
                "index": "G",
                "name": "NPC Challenge",
                "type": "PROGRAMMING",
                "points": 3500.0,
                "rating": 3000,
                "tags": ["divide and conquer", "interactive"]
            },
            "author": {
                "contestId": 2245,
                "members": [{"handle": "tourist"}],
                "participantType": "CONTESTANT",
                "ghost": false
            },
            "programmingLanguage": "C++23 (GCC 14-64, msys2)",
            "verdict": "OK",
            "passedTestCount": 39
        }"#;

        let sub: Submission = serde_json::from_str(json).expect("deserialize submission");
        assert_eq!(sub.id, 383013765);
        assert_eq!(sub.contest_id, Some(2245));
        assert_eq!(sub.creation_time_seconds, 1784221884);
        assert_eq!(sub.programming_language, "C++23 (GCC 14-64, msys2)");
        assert_eq!(sub.verdict.as_deref(), Some("OK"));
        assert_eq!(sub.passed_test_count, 39);
        assert_eq!(sub.problem.problem_type, "PROGRAMMING");
        assert_eq!(sub.problem.code(), "2245G");
    }

    #[test]
    fn test_contest_and_standings_deserialization() {
        let json = r#"{
            "contest": {
                "id": 566,
                "name": "VK Cup 2015",
                "type": "CF",
                "phase": "FINISHED",
                "durationSeconds": 10800,
                "startTimeSeconds": 1438273200
            },
            "problems": [
                {
                    "contestId": 566,
                    "index": "A",
                    "name": "Matching Names",
                    "type": "PROGRAMMING",
                    "points": 1750.0,
                    "rating": 2300,
                    "tags": ["dfs and similar"]
                }
            ],
            "rows": [
                {
                    "party": {
                        "members": [{"handle": "rng_58"}],
                        "participantType": "CONTESTANT",
                        "ghost": false
                    },
                    "rank": 1,
                    "points": 7974.0,
                    "penalty": 0,
                    "successfulHackCount": 1,
                    "unsuccessfulHackCount": 0,
                    "problemResults": [
                        {
                            "points": 1330.0,
                            "rejectedAttemptCount": 0,
                            "type": "FINAL",
                            "bestSubmissionTimeSeconds": 3624
                        }
                    ]
                }
            ]
        }"#;

        let standings: ContestStandingsResult =
            serde_json::from_str(json).expect("deserialize standings");
        assert_eq!(standings.contest.id, 566);
        assert_eq!(standings.contest.contest_type, "CF");
        assert_eq!(standings.contest.duration_seconds, 10800);
        assert_eq!(standings.rows.len(), 1);
        assert_eq!(standings.rows[0].position, 1);
        assert_eq!(standings.rows[0].successful_hack_count, 1);
        assert_eq!(standings.rows[0].problem_results.len(), 1);
        assert_eq!(
            standings.rows[0].problem_results[0].problem_result_type,
            "FINAL"
        );
    }

    #[test]
    fn test_unrated_user_defaults() {
        let json = r#"{
            "handle": "newbie_user",
            "contribution": 0,
            "lastOnlineTimeSeconds": 1525933227,
            "friendOfCount": 0,
            "titlePhoto": "https://userpic.codeforces.org/no-title.jpg",
            "avatar": "https://userpic.codeforces.org/no-avatar.jpg",
            "registrationTimeSeconds": 1513795921
        }"#;

        let user: User = serde_json::from_str(json).expect("deserialize unrated user");
        assert_eq!(user.handle, "newbie_user");
        assert_eq!(user.rating, None);
        assert_eq!(user.rank, None);
        assert_eq!(user.first_name, None);
        assert_eq!(user.last_name, None);
        assert_eq!(user.full_name(), None);
        assert_eq!(
            user.photo_url(),
            "https://userpic.codeforces.org/no-title.jpg"
        );
    }

    #[test]
    fn test_blog_comment_recent_action() {
        let json = r#"{
            "timeSeconds": 1788018175,
            "blogEntry": {
                "id": 156291,
                "title": "<p>AtCoder Beginner Contest 473 Announcement</p>",
                "authorHandle": "atcoder_official",
                "creationTimeSeconds": 1787914272,
                "rating": -12,
                "tags": []
            },
            "comment": {
                "id": 1389033,
                "creationTimeSeconds": 1788018175,
                "commentatorHandle": "EchoHua0402",
                "text": "<p>hello</p>",
                "rating": 0
            }
        }"#;

        let ra: RecentAction = serde_json::from_str(json).expect("deserialize recent action");
        assert_eq!(ra.time_seconds, 1788018175);
        let b = ra.blog_entry.expect("blog entry present");
        assert_eq!(b.id, 156291);
        assert_eq!(b.author_handle, "atcoder_official");
        assert_eq!(b.rating, Some(-12));
        let c = ra.comment.expect("comment present");
        assert_eq!(c.id, 1389033);
        assert_eq!(c.commentator_handle, "EchoHua0402");
    }

    #[test]
    fn test_problemset_result_deserialization() {
        let json = r#"{
            "problems": [
                {
                    "contestId": 2257,
                    "index": "F2",
                    "name": "Beaver Track",
                    "type": "PROGRAMMING",
                    "points": 1000.0,
                    "rating": 2700,
                    "tags": ["data structures", "dp"]
                }
            ],
            "problemStatistics": [
                {
                    "contestId": 2257,
                    "index": "F2",
                    "solvedCount": 45
                }
            ]
        }"#;

        let res: ProblemSetResult = serde_json::from_str(json).expect("deserialize problemset");
        assert_eq!(res.problems.len(), 1);
        assert_eq!(res.problem_statistics.len(), 1);
        assert_eq!(res.problems[0].index, "F2");
        assert_eq!(res.problem_statistics[0].solved_count, 45);
    }
}
