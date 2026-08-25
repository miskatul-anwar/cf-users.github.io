//! Small helpers: rank colors, date formatting, HTML stripping.

/// Official Codeforces rank colors.
pub fn rank_color(rank: &str) -> &'static str {
    match rank.to_lowercase().as_str() {
        "newbie" => "#808080",
        "pupil" => "#008000",
        "specialist" => "#03a89e",
        "expert" => "#0000ff",
        "candidate master" => "#aa00aa",
        "master" | "international master" => "#ff8c00",
        "grandmaster" | "international grandmaster" => "#ff0000",
        "legendary grandmaster" => "#ff0000",
        _ => "#000000",
    }
}

/// Color for an arbitrary rating value using the Codeforces palette.
pub fn rating_color(rating: i32) -> &'static str {
    match rating {
        0..=1199 => "#808080",
        1200..=1399 => "#008000",
        1400..=1599 => "#03a89e",
        1600..=1899 => "#0000ff",
        1900..=2099 => "#aa00aa",
        2100..=2299 => "#ff8c00",
        2300..=2399 => "#ff8c00",
        _ => "#ff0000",
    }
}

/// Color for a submission verdict string from the API.
pub fn verdict_color(verdict: &str) -> &'static str {
    match verdict {
        "OK" => "#008000",
        "WRONG_ANSWER"
        | "COMPILATION_ERROR"
        | "RUNTIME_ERROR"
        | "IDLENESS_LIMIT_EXCEEDED"
        | "PRESENTATION_ERROR"
        | "CHALLENGED"
        | "SKIPPED"
        | "TESTING"
        | "REJECTED" => "#ff0000",
        "TIME_LIMIT_EXCEEDED" | "MEMORY_LIMIT_EXCEEDED" => "#ff8c00",
        "PARTIAL" => "#aa00aa",
        _ => "#808080",
    }
}

/// Human readable verdict text ("OK", "Wrong answer", ...).
pub fn verdict_text(verdict: &str) -> String {
    match verdict {
        "OK" => "Accepted".into(),
        "WRONG_ANSWER" => "Wrong answer".into(),
        "TIME_LIMIT_EXCEEDED" => "Time limit exceeded".into(),
        "MEMORY_LIMIT_EXCEEDED" => "Memory limit exceeded".into(),
        "COMPILATION_ERROR" => "Compilation error".into(),
        "RUNTIME_ERROR" => "Runtime error".into(),
        "IDLENESS_LIMIT_EXCEEDED" => "Idleness limit exceeded".into(),
        "PRESENTATION_ERROR" => "Presentation error".into(),
        "CHALLENGED" => "Hacked".into(),
        "SKIPPED" => "Skipped".into(),
        "TESTING" => "In testing".into(),
        "REJECTED" => "Rejected".into(),
        "PARTIAL" => "Partial".into(),
        other => other.replace('_', " "),
    }
}

/// Format unix seconds as "YYYY-MM-DD HH:MM" (UTC).
pub fn format_time(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (h, m) = (rem / 3600, (rem % 3600) / 60);
    let (y, mo, d) = civil_from_days(days);
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{m:02}")
}

/// Format unix seconds as "YYYY-MM-DD".
pub fn format_date(secs: i64) -> String {
    let (y, mo, d) = civil_from_days(secs.div_euclid(86_400));
    format!("{y:04}-{mo:02}-{d:02}")
}

/// Days-since-epoch to proleptic Gregorian date (Howard Hinnant's algorithm).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Format a duration in seconds as e.g. "2d 05:30" or "45m".
pub fn format_duration(secs: i64) -> String {
    let d = secs / 86_400;
    let h = (secs % 86_400) / 3600;
    let m = (secs % 3600) / 60;
    if d > 0 {
        format!("{d}d {h:02}:{m:02}")
    } else {
        format!("{h:02}:{m:02}")
    }
}

/// Signed delta with color-friendly sign, e.g. "+42" or "-17".
pub fn signed_delta(delta: i32) -> String {
    if delta >= 0 {
        format!("+{delta}")
    } else {
        delta.to_string()
    }
}

/// Thousands separators, e.g. 12345 -> "12,345".
pub fn thousands(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

/// Very small tag stripper so raw API HTML is readable as plain text.
pub fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut inside = false;
    for ch in html.chars() {
        match ch {
            '<' => inside = true,
            '>' => inside = false,
            c if !inside => out.push(c),
            _ => {}
        }
    }
    // Collapse whitespace runs and common entities.
    let out = out.replace("&nbsp;", " ");
    let out = out
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Truncate a string on char boundaries.
pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max).collect();
        format!("{cut}\u{2026}")
    }
}
