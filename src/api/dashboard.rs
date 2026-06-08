//! Minimal but usable HTML dashboard. The JS polls `/api/summary`
//! every 2 seconds and renders an approval queue. We embed everything
//! inline so deployment is a single binary (no static files needed).

use axum::extract::Path;
use axum::http::header;
use axum::response::{Html, IntoResponse};

const PAGE_CSS: &str = include_str!("../../assets/dashboard.css");
const PAGE_JS: &str = include_str!("../../assets/dashboard.js");
const DRIVER_JS: &str = include_str!("../../assets/driver.js");
const DRIVER_CSS: &str = "https://cdn.jsdelivr.net/npm/driver.js@1.3.1/dist/driver.css";

pub async fn index() -> Html<String> {
    Html(format!(
        r##"<!doctype html>
<html><head><meta charset="utf-8"><title>Toran</title>
<meta name="viewport" content="width=device-width,initial-scale=1">
<link rel="stylesheet" href="{driver_css}">
<style>{css}</style></head>
<body><header><h1>Toran</h1>
<p class="tag">Runtime human-approval gatekeeper for AI agents.</p>
<nav><a href="/dashboard">Dashboard</a> &middot; <a href="/dashboard/audit">Audit Log</a> &middot; <a href="/dashboard/policies">Policies</a> &middot; <a href="/api/health">Health</a> &middot; <a href="/api/metrics">Metrics</a> &middot; <a href="#" id="start-tour">Take the tour</a></nav>
</header><main><section><h2>What is Toran?</h2>
<p>Toran is a gatekeeper. It sits between your AI agent and the real world. When the agent tries to send an email, write to a database, or call an API, Toran checks a <a href="/dashboard/policies">policy</a>. If the action is allowed, it executes immediately. If it is risky, Toran pauses and asks a human for approval via the <a href="/dashboard">approval queue</a>.</p>
<h2>Quick start</h2>
<ol>
<li>Drop the <code>@gate</code> decorator on any Python function. See the <a href="https://github.com/">README</a>.</li>
<li>Put a YAML policy file in <code>./policies/</code> (the path the core is watching).</li>
<li>Run your agent. When it hits a <code>REQUIRE_APPROVAL</code> rule, it appears here.</li>
</ol>
<h2>Status</h2>
<div id="home-status">loading&hellip;</div>
</section></main>
<script>{driver_js}</script>
<script>{js}</script>
<script>toran.homeStatus(); toran.tour.bindHome();</script>
</body></html>"##,
        css = PAGE_CSS,
        js = PAGE_JS,
        driver_js = DRIVER_JS,
        driver_css = DRIVER_CSS
    ))
}

pub async fn dashboard() -> Html<String> {
    Html(format!(
        r##"<!doctype html>
<html><head><meta charset="utf-8"><title>Toran &middot; Dashboard</title>
<link rel="stylesheet" href="{driver_css}">
<style>{css}</style></head>
<body><header><h1>Toran &middot; Approval Queue</h1>
<nav><a href="/">Home</a> &middot; <a href="/dashboard">Queue</a> &middot; <a href="/dashboard/audit">Audit</a> &middot; <a href="/dashboard/policies">Policies</a> &middot; <a href="/api/health">Health</a> &middot; <a href="/api/metrics">Metrics</a> &middot; <a href="#" id="start-tour">Take the tour</a></nav>
</header><main>
<section class="cards" id="cards">
  <div class="card"><div class="card-num" id="m-pending">&ndash;</div><div class="card-lbl">Pending</div></div>
  <div class="card"><div class="card-num" id="m-decisions">&ndash;</div><div class="card-lbl">Total decisions</div></div>
  <div class="card"><div class="card-num" id="m-avg">&ndash;</div><div class="card-lbl">Avg eval (ns)</div></div>
  <div class="card"><div class="card-num" id="m-uptime">&ndash;</div><div class="card-lbl">Uptime (s)</div></div>
</section>
<section id="queue-section"><h2>Pending approvals</h2>
<table class="queue"><thead><tr>
<th>Created</th><th>Function</th><th>Agent</th><th>Risk</th><th>Rule</th><th></th>
</tr></thead><tbody id="rows"></tbody></table>
<p id="empty" class="empty">No pending approvals.</p></section>
</main>
<script>{driver_js}</script>
<script>{js}</script>
<script>toran.startDashboard(); toran.tour.bindDashboard();</script>
</body></html>"##,
        css = PAGE_CSS,
        js = PAGE_JS,
        driver_js = DRIVER_JS,
        driver_css = DRIVER_CSS
    ))
}

pub async fn approval_detail(Path(id): Path<String>) -> impl IntoResponse {
    let html = format!(
        r##"<!doctype html>
<html><head><meta charset="utf-8"><title>Toran &middot; Approval {id}</title>
<link rel="stylesheet" href="{driver_css}">
<style>{css}</style></head>
<body><header><h1>Approval {id}</h1>
<nav><a href="/dashboard">&larr; Back to queue</a> &middot; <a href="#" id="start-tour">Take the tour</a></nav></header>
<main><section id="detail">loading&hellip;</section>
<section id="resolve-section"><h2>Resolve</h2>
<p>To resolve programmatically, POST JSON to:</p>
<pre>curl -X POST http://localhost:7878/api/approvals/{id}/approve \
  -H 'content-type: application/json' \
  -d '{{"resolved_by":"me","token":"&lt;notify_token&gt;"}}'</pre>
<button id="btn-approve" onclick="toran.resolve('{id}','approve')">Approve</button>
<button class="danger" id="btn-deny" onclick="toran.resolve('{id}','deny')">Deny</button>
</section></main>
<script>{driver_js}</script>
<script>{js}</script>
<script>toran.loadDetail('{id}'); toran.tour.bindDetail('{id}');</script>
</body></html>"##,
        css = PAGE_CSS,
        js = PAGE_JS,
        driver_js = DRIVER_JS,
        driver_css = DRIVER_CSS,
        id = id
    );
    ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], html)
}

pub async fn audit() -> Html<String> {
    Html(format!(
        r##"<!doctype html>
<html><head><meta charset="utf-8"><title>Toran &middot; Audit Log</title>
<link rel="stylesheet" href="{driver_css}">
<style>{css}</style></head>
<body><header><h1>Audit Log</h1>
<nav><a href="/dashboard">&larr; Queue</a> &middot; <a href="/api/audit?limit=500">JSON</a> &middot; <a href="#" id="start-tour">Take the tour</a></nav></header>
<main><table class="queue" id="audit-table"><thead><tr>
<th>Time</th><th>Event</th><th>Function</th><th>Agent</th><th>Decision</th><th>Rule</th>
</tr></thead><tbody id="rows"></tbody></table></main>
<script>{driver_js}</script>
<script>{js}</script>
<script>toran.startAudit(); toran.tour.bindAudit();</script>
</body></html>"##,
        css = PAGE_CSS,
        js = PAGE_JS,
        driver_js = DRIVER_JS,
        driver_css = DRIVER_CSS
    ))
}

pub async fn policies() -> Html<String> {
    Html(format!(
        r##"<!doctype html>
<html><head><meta charset="utf-8"><title>Toran &middot; Policies</title>
<link rel="stylesheet" href="{driver_css}">
<style>{css}</style></head>
<body><header><h1>Policies</h1>
<nav><a href="/dashboard">&larr; Queue</a> &middot; <a href="#" id="start-tour">Take the tour</a></nav></header>
<main><div id="policies">loading&hellip;</div></main>
<script>{driver_js}</script>
<script>{js}</script>
<script>toran.startPolicies(); toran.tour.bindPolicies();</script>
</body></html>"##,
        css = PAGE_CSS,
        js = PAGE_JS,
        driver_js = DRIVER_JS,
        driver_css = DRIVER_CSS
    ))
}
