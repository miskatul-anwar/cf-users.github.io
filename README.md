# Codeforces Explorer

A comprehensive client-side explorer for the [Codeforces API](https://codeforces.com/api/help), built with [Leptos](https://leptos.dev) (CSR WASM) and [Thaw UI](https://github.com/thaw-ui/thaw).

![Work Flow](pngs/main.png)

## Features

Every public (unauthenticated) Codeforces API endpoint is supported:

| Tab | Features | Endpoints |
| --- | --- | --- |
| **User** | Profile cards for multiple handles at once, rating history chart + stats, recent submissions with count selector, blog entries with full-text viewer, comment history | `user.info`, `user.rating`, `user.status`, `user.blogEntries`, `user.comments`, `blogEntry.view`, `blogEntry.comments` |
| **Contests** | Full contest list with phase/name filters and pagination; per-contest inspector with standings (rank/handles/unofficial filters), rating changes summary, hacks feed, submission browser | `contest.list`, `contest.standings`, `contest.ratingChanges`, `contest.hacks`, `contest.status` |
| **Problems** | Whole problem set with tag / rating-range / name filters, solved-count sorting, pagination | `problemset.problems` |
| **Recent** | Global recent-actions feed and the complete rated list with search + pagination | `recentActions`, `user.ratedList` |

Extras: official rank colors everywhere, SVG rating-history chart, colored verdicts/deltas, deep links to codeforces.com.

## Development

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk
trunk serve        # dev server with hot reload
```

## Deployment

CI builds with Trunk (`trunk build --release`) and deploys `dist/` to Vercel on every push to `main`. Pull requests get preview deployments.
