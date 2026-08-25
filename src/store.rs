//! Global app state: hash router, persisted theme and a shared problemset.

use crate::{api, storage};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cell::RefCell;
use thaw::Theme;

// ---------------------------------------------------------------------------
// Router (shareable deep links like #/user/tourist)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum Route {
    User(Option<String>),
    Compare(Vec<String>),
    Contests,
    Problems,
    Recent,
}

fn parse_hash(hash: &str) -> Route {
    let segs: Vec<String> = hash
        .trim_start_matches('#')
        .split('/')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    match segs.first().map(String::as_str) {
        Some("compare") => Route::Compare(segs.into_iter().skip(1).collect()),
        Some("contests") => Route::Contests,
        Some("problems") => Route::Problems,
        Some("recent") => Route::Recent,
        _ => Route::User(segs.into_iter().nth(1)),
    }
}

fn hash_for(route: &Route) -> String {
    match route {
        Route::User(None) => "#/user".into(),
        Route::User(Some(h)) => format!("#/user/{}", h.replace('/', "")),
        Route::Compare(v) if v.is_empty() => "#/compare".into(),
        Route::Compare(v) => format!(
            "#/compare/{}",
            v.iter()
                .map(|h| h.replace('/', ""))
                .collect::<Vec<_>>()
                .join("/")
        ),
        Route::Contests => "#/contests".into(),
        Route::Problems => "#/problems".into(),
        Route::Recent => "#/recent".into(),
    }
}

fn current_hash() -> String {
    window().location().hash().ok().unwrap_or_default()
}

thread_local! {
    static ROUTE: RefCell<Option<RwSignal<Route>>> = const { RefCell::new(None) };
}

/// Current navigation route; initializes from the URL hash exactly once.
pub fn route() -> RwSignal<Route> {
    ROUTE.with(|r| {
        let mut slot = r.borrow_mut();
        if let Some(sig) = slot.as_ref() {
            return *sig;
        }
        let initial = parse_hash(&current_hash());
        let sig = RwSignal::new(initial);
        let listener = sig;
        listen_event("hashchange", move || {
            listener.set(parse_hash(&current_hash()));
        });
        *slot = Some(sig);
        sig
    })
}

/// Navigate to a route, updating the URL hash (pushes a history entry).
pub fn go(target: Route) {
    let hash = hash_for(&target);
    if current_hash() == hash {
        // Avoid pointless notifications that would remount the view.
        if route().get_untracked() != target {
            route().set(target);
        }
    } else {
        let _ = window().location().set_hash(&hash);
    }
}

fn listen_event(name: &str, f: impl Fn() + 'static) {
    use leptos::wasm_bindgen::{JsCast, closure::Closure};
    let cb = Closure::wrap(Box::new(f) as Box<dyn Fn()>);
    let _ = window().add_event_listener_with_callback(name, cb.as_ref().unchecked_ref());
    cb.forget();
}

// ---------------------------------------------------------------------------
// Theme (persisted)
// ---------------------------------------------------------------------------

thread_local! {
    static THEME: RefCell<Option<RwSignal<Theme>>> = const { RefCell::new(None) };
    static DARK: RefCell<bool> = const { RefCell::new(false) };
}

/// The global theme signal, initialized from localStorage.
pub fn theme() -> RwSignal<Theme> {
    THEME.with(|t| {
        let mut slot = t.borrow_mut();
        if let Some(sig) = slot.as_ref() {
            return *sig;
        }
        let dark = storage::ls_get("cfx-theme").as_deref() == Some("dark");
        DARK.with(|d| *d.borrow_mut() = dark);
        let sig = RwSignal::new(if dark { Theme::dark() } else { Theme::light() });
        *slot = Some(sig);
        sig
    })
}

pub fn is_dark() -> bool {
    DARK.with(|d| *d.borrow())
}

pub fn toggle_theme() {
    let next_dark = !is_dark();
    DARK.with(|d| *d.borrow_mut() = next_dark);
    storage::ls_set("cfx-theme", if next_dark { "dark" } else { "light" });
    theme().set(if next_dark {
        Theme::dark()
    } else {
        Theme::light()
    });
}

// ---------------------------------------------------------------------------
// Shared problemset (fetched once, reused by Problems tab and recommendations)
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
pub enum SharedProblemset {
    #[default]
    Loading,
    Error(String),
    Ready(std::sync::Arc<Vec<api::Problem>>),
}

thread_local! {
    static PROBLEMSET: RefCell<Option<RwSignal<SharedProblemset>>> = const { RefCell::new(None) };
}

/// Lazily-fetched full problem set shared across views.
pub fn problemset() -> RwSignal<SharedProblemset> {
    PROBLEMSET.with(|p| {
        let mut slot = p.borrow_mut();
        if let Some(sig) = slot.as_ref() {
            return *sig;
        }
        let sig = RwSignal::new(SharedProblemset::Loading);
        spawn_local(async move {
            match api::problemset_problems_cached().await {
                Ok(res) => sig.set(SharedProblemset::Ready(std::sync::Arc::new(res.problems))),
                Err(e) => sig.set(SharedProblemset::Error(e)),
            }
        });
        *slot = Some(sig);
        sig
    })
}
