# RFC: devflow — Terminal DORA Metrics for GitHub

| Field | Value |
|---|---|
| **Status** | Draft |
| **Author** | Kumar Saheb |
| **Date** | July 6, 2026 |
| **Audience** | Engineering leadership, Platform/DevOps, IC engineers |
| **Org context** | CreditGenie already uses **Jellyfish** for engineering metrics |

---

## Summary

Propose **devflow** as a **complementary** tool — not a Jellyfish replacement.

**Jellyfish** remains the system of record for org-wide DORA reporting, investment tracking, and leadership dashboards.

**devflow** fills a different niche: a **free, local, terminal-native** way for engineers and leads to **quickly inspect GitHub delivery health** without opening a browser, without waiting on dashboard setup, and without sending ad-hoc queries through a SaaS UI.

---

## Context: We Already Have Jellyfish

CreditGenie's stack already includes Jellyfish (alongside Datadog, PagerDuty, GitHub Enterprise, Jeli, etc.). Jellyfish is the right tool for:

- Executive and manager reporting
- Longitudinal trends and investment allocation
- Org-wide benchmarks and team comparisons
- Product/engineering alignment views

**This RFC does not propose replacing Jellyfish.**

Instead, it proposes devflow where Jellyfish is the wrong shape of tool.

---

## Problem (Reframed)

Even with Jellyfish, teams still hit friction:

| Gap | Jellyfish | devflow |
|---|---|---|
| Engineer wants a **30-second health check** before standup | Browser login, navigate dashboards | `./devflow` in a terminal pane |
| **Ad-hoc slice** — "just these 3 repos, last 2 weeks" | Filter UI, saved views, permissions | `GITHUB_REPOS=... DEVFLOW_LOOKBACK_DAYS=14 ./devflow` |
| **Local / private** query on sensitive repo activity | Data in Jellyfish cloud | Runs on laptop; data never leaves your machine |
| **No Jellyfish seat** (contractor, new hire, OSS contributor) | No access | Own GitHub token + binary |
| **CI/debug** — "is the fetcher broken or is there no data?" | Hard to tell from UI | `RUST_LOG=info ./devflow` shows per-repo fetch counts |
| **Offline-ish workflow** | Requires web app | TUI after initial GitHub fetch |

The problem is not "we lack DORA metrics." The problem is **access latency and workflow fit** for people who live in the terminal and need fast, scoped answers.

---

## Proposal

Ship **devflow** as:

1. **Internal complement at CreditGenie** — optional CLI for engineers, platform team, and EM ad-hoc checks
2. **Open-source project** — useful for teams that do *not* have Jellyfish (startups, OSS maintainers, cost-sensitive orgs)

### What devflow is

- Single Rust binary
- Reads GitHub REST API (PRs, Actions runs, Deployments when present)
- Computes DORA + bonus metrics (cycle time, review pickup, CI pass rate)
- Ratatui dashboard: Overview · Repos · Authors · CI

### What devflow is not

- Not a Jellyfish competitor for leadership reporting
- Not a historical analytics warehouse
- Not a sprint/planning or investment allocation tool
- Not a replacement for Datadog incident/APM correlation

---

## Jellyfish vs devflow — When to Use Which

| Scenario | Use |
|---|---|
| Quarterly eng review, headcount/investment reporting | **Jellyfish** |
| Compare teams over quarters, benchmark vs industry | **Jellyfish** |
| Manager dashboard for direct reports | **Jellyfish** |
| Engineer checking "did our CI get worse this week?" | **devflow** |
| Platform team debugging GitHub API / repo list / SAML token issues | **devflow** |
| Quick DORA snapshot before incident postmortem | **devflow** |
| Scoped analysis: one service, one squad's repos | **devflow** |
| Demo at meetup / OSS project without enterprise tooling | **devflow** |

**Rule of thumb:** Jellyfish for **decisions and reporting**; devflow for **inspection and debugging**.

---

## Architecture (unchanged)

```
GitHub REST API
      │
      ▼
devflow-core   ← fetch (parallel, capped repos) + DORA engine
      │
      ▼
devflow TUI    ← Overview / Repos / Authors / CI
```

**Design choices that complement Jellyfish:**

- **No database** — always fresh from GitHub; good for "what's true right now?"
- **Explicit proxies** — merged PRs → main when Deployments API unused; shows "No data" honestly
- **Repo cap (50 most recently pushed)** — fast default for large orgs like CreditGenie (355 repos)
- **Env-based scoping** — `GITHUB_REPOS` for surgical queries Jellyfish filters handle differently

---

## Metrics

| Metric | Source | Notes |
|---|---|---|
| Deployment frequency | Deployments API **or** merged PRs to `main` | Proxy labeled in UI |
| Lead time for changes | PR open → merge | Median |
| Change failure rate | Failed vs successful deployments | "No data" if no deploy API usage |
| MTTR | Failure → next success | Same |
| PR cycle time | Bonus | |
| Review turnaround | Bonus | Needs review timestamp enrichment |
| CI pass rate | GitHub Actions | |

DORA level bands follow Google DORA research (Elite / High / Medium / Low / No data).

---

## CreditGenie-Specific Considerations

### Already solved by Jellyfish

- Org-wide DORA trends for leadership
- Team-level reporting and benchmarks
- Integration with existing eng management workflows

### Still useful for CreditGenie

- **Engineers** — terminal workflow, no context switch during review/debug sessions
- **Platform/DevOps** — validate GitHub token/SAML, repo scope, API rate limits before blaming "no metrics"
- **EMs** — quick pre-meeting snapshot for a subset of repos (e.g. mobile squad only)
- **On-call / incidents** — fast "what merged recently / CI status" without opening Jellyfish + Datadog separately

### Known limitations (same as before)

- GitHub fine-grained tokens require org SSO authorization
- Large org fetch can be slow without `GITHUB_REPOS` scoping
- No persisted history — cannot replace Jellyfish trend charts

---

## Competitive Landscape (Updated)

| Tool | Role relative to CreditGenie |
|---|---|
| **Jellyfish** | Primary eng metrics platform — keep |
| **Datadog** | APM, infra, optional DORA with CI Visibility — keep |
| **devflow** | Terminal complement + OSS — add optionally |
| LinearB / Swarmia | Jellyfish alternatives — not needed |

---

## Implementation Status

- [x] Rust workspace (`devflow-core`, `devflow`)
- [x] GitHub client with parallel fetch + repo cap
- [x] DORA engine with honest `NoData` levels
- [x] TUI: Overview, Repos, Authors, CI
- [x] Demo mode (`--demo`)
- [x] Validated against CreditGenie org (SAML SSO token flow documented)
- [ ] Local snapshot cache (SQLite) for faster repeat runs — optional
- [ ] Review timestamp enrichment from GitHub Reviews API
- [ ] `brew install` / GitHub Releases polish
- [ ] Optional: weekly Slack digest via GitHub Action (does **not** duplicate Jellyfish; could post to `#eng-platform` for repo-scoped alerts only)

---

## Recommendation

### For CreditGenie (internal)

| Option | Recommendation |
|---|---|
| Replace Jellyfish with devflow | **Reject** — Jellyfish already covers reporting |
| Adopt devflow as engineer-side complement | **Approve (optional)** — low cost, no license, useful for terminal workflow |
| Mandate devflow for all EMs | **Reject** — Jellyfish remains source of truth for leadership |

**Suggested internal policy:**

> Jellyfish is the official system of record for engineering metrics and leadership reporting.  
> devflow is an optional CLI for ad-hoc, repo-scoped, terminal-based inspection. Numbers may differ slightly from Jellyfish due to methodology (e.g. PR-as-deploy proxy); do not use devflow alone for executive reporting.

### For open source (external)

**Approve** publishing devflow as OSS for teams without Jellyfish — clear positioning vs paid EM platforms.

---

## Open Questions

1. Should CreditGenie host an internal `GITHUB_REPOS` preset for common squad scopes (mobile, backend, infra)?
2. Is there value in a `--json` export mode for piping into Snowflake/ad-hoc analysis (complementing Jellyfish, not replacing)?
3. Who owns internal docs: Platform team or Eng Productivity?

---

## Decision

- [ ] **Approve** — optional internal complement + open-source
- [ ] **Approve OSS only** — no internal promotion
- [ ] **Request changes**
- [ ] **Reject**

**Approver:** _______________  
**Date:** _______________

---

## Appendix: Quick Start (CreditGenie)

```bash
# .env
GITHUB_TOKEN=ghp_...          # classic token, SSO authorized for CreditGenie
GITHUB_OWNER=CreditGenie
GITHUB_REPOS=                 # optional: narrow for speed
DEVFLOW_LOOKBACK_DAYS=90

./devflow --demo              # fake data
RUST_LOG=info ./devflow       # real org (may take ~15s for 50 repos)
```
