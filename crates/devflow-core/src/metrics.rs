use chrono::{DateTime, Duration, Utc};

use crate::types::*;

/// Calculate all DORA metrics from a raw dataset.
pub fn calculate(ds: &Dataset, lookback_days: u32) -> DoraMetrics {
    let window = lookback_days as f64;
    let now    = Utc::now();
    let since  = now - Duration::days(lookback_days as i64);

    // Slice to window
    let prs: Vec<&PullRequest> = ds.prs.iter()
        .filter(|p| p.created_at >= since)
        .collect();

    // Use GitHub Deployments if present, otherwise fall back to merged-to-main PRs
    let has_deployments = ds.deployments.iter().any(|d| d.created_at >= since);
    let deployments: Vec<&Deployment> = ds.deployments.iter()
        .filter(|d| d.created_at >= since && d.status == DeploymentStatus::Success)
        .collect();
    let failures: Vec<&Deployment> = ds.deployments.iter()
        .filter(|d| d.created_at >= since && d.status == DeploymentStatus::Failure)
        .collect();
    let ci_runs: Vec<&WorkflowRun> = ds.ci_runs.iter()
        .filter(|r| r.created_at >= since)
        .collect();

    // Proxy: merged PRs to main/master as deploy events when no Deployments API data
    let merged_to_main: Vec<&PullRequest> = prs.iter()
        .copied()
        .filter(|p| p.merged_at.is_some() &&
            (p.base_ref == "main" || p.base_ref == "master"))
        .collect();

    DoraMetrics {
        deployment_frequency:  if has_deployments {
            deployment_frequency(&deployments, window, since, now)
        } else {
            deployment_frequency_from_prs(&merged_to_main, window, since, now)
        },
        lead_time:             lead_time_for_changes(&prs),
        change_failure_rate:   if has_deployments {
            change_failure_rate(&deployments, &failures, since, now)
        } else {
            RateMetric { rate: 0.0, level: DoraLevel::NoData, history: vec![] }
        },
        mean_time_to_recovery: if has_deployments {
            mttr(&failures, &deployments, since, now)
        } else {
            DurationMetric { median_hours: 0.0, level: DoraLevel::NoData, history: vec![] }
        },
        pr_cycle_time:         pr_cycle_time(&prs),
        review_turnaround:     review_turnaround(&prs),
        ci_pass_rate:          ci_pass_rate(&ci_runs, since, now),
        median_pr_size:        median_pr_size(&prs),
    }
}

/// Per-repo stats
pub fn per_repo(ds: &Dataset, lookback_days: u32) -> Vec<RepoStats> {
    let since = Utc::now() - Duration::days(lookback_days as i64);
    let repos: std::collections::HashSet<&str> = ds.prs.iter()
        .map(|p| p.repo.as_str())
        .collect();

    let mut stats: Vec<RepoStats> = repos.into_iter().map(|repo| {
        let repo_prs: Vec<&PullRequest> = ds.prs.iter()
            .filter(|p| p.repo == repo && p.created_at >= since)
            .collect();
        let merged: Vec<&PullRequest> = repo_prs.iter()
            .copied()
            .filter(|p| p.merged_at.is_some())
            .collect();
        let deploys = ds.deployments.iter()
            .filter(|d| d.repo == repo && d.created_at >= since && d.status == DeploymentStatus::Success)
            .count();
        let ci = ds.ci_runs.iter()
            .filter(|r| r.repo == repo && r.created_at >= since)
            .collect::<Vec<_>>();

        RepoStats {
            repo:             repo.to_owned(),
            pr_count:         repo_prs.len(),
            merged_count:     merged.len(),
            avg_cycle_hours:  avg_cycle_hours(&merged),
            avg_review_hours: avg_review_hours(&repo_prs),
            ci_pass_rate:     pass_rate_of(&ci),
            deploy_count:     deploys,
        }
    }).collect();

    stats.sort_by(|a, b| b.pr_count.cmp(&a.pr_count));
    stats
}

/// Per-author stats
pub fn per_author(ds: &Dataset, lookback_days: u32) -> Vec<AuthorStats> {
    let since = Utc::now() - Duration::days(lookback_days as i64);
    let authors: std::collections::HashSet<&str> = ds.prs.iter()
        .map(|p| p.author.as_str())
        .collect();

    let mut stats: Vec<AuthorStats> = authors.into_iter().map(|author| {
        let author_prs: Vec<&PullRequest> = ds.prs.iter()
            .filter(|p| p.author == author && p.created_at >= since)
            .collect();
        let merged: Vec<&PullRequest> = author_prs.iter()
            .copied()
            .filter(|p| p.merged_at.is_some())
            .collect();
        let avg_size = if author_prs.is_empty() { 0.0 } else {
            author_prs.iter().map(|p| (p.additions + p.deletions) as f64).sum::<f64>()
                / author_prs.len() as f64
        };
        AuthorStats {
            author:           author.to_owned(),
            pr_count:         author_prs.len(),
            merged_count:     merged.len(),
            avg_cycle_hours:  avg_cycle_hours(&merged),
            avg_pr_size:      avg_size,
            reviews_given:    0, // populated later when review data is fetched
        }
    }).collect();

    stats.sort_by(|a, b| b.pr_count.cmp(&a.pr_count));
    stats
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn deployment_frequency(
    deploys: &[&Deployment],
    window:  f64,
    since:   DateTime<Utc>,
    now:     DateTime<Utc>,
) -> FrequencyMetric {
    if deploys.is_empty() {
        return FrequencyMetric { per_day: 0.0, level: DoraLevel::NoData, history: vec![] };
    }
    freq_from_count(deploys.len(), window, |age| {
        let days = window as usize;
        let mut b = vec![0f64; days];
        for d in deploys {
            let a = (now - d.created_at).num_days() as usize;
            if a < days { b[days - 1 - a] += 1.0; }
        }
        b
    })
}

fn deployment_frequency_from_prs(
    prs:    &[&PullRequest],
    window: f64,
    since:  DateTime<Utc>,
    now:    DateTime<Utc>,
) -> FrequencyMetric {
    if prs.is_empty() {
        return FrequencyMetric { per_day: 0.0, level: DoraLevel::NoData, history: vec![] };
    }
    let days = window as usize;
    let mut buckets = vec![0f64; days];
    for p in prs {
        if let Some(m) = p.merged_at {
            let age = (now - m).num_days() as usize;
            if age < days { buckets[days - 1 - age] += 1.0; }
        }
    }
    freq_from_count(prs.len(), window, |_| buckets)
}

fn freq_from_count(
    count:   usize,
    window:  f64,
    history: impl FnOnce(usize) -> Vec<f64>,
) -> FrequencyMetric {
    let per_day = count as f64 / window;
    let level = if per_day >= 1.0         { DoraLevel::Elite  }
               else if per_day >= 1.0/7.0  { DoraLevel::High   }
               else if per_day >= 1.0/30.0 { DoraLevel::Medium }
               else                        { DoraLevel::Low    };
    FrequencyMetric { per_day, level, history: history(0) }
}

fn lead_time_for_changes(prs: &[&PullRequest]) -> DurationMetric {
    let mut hours: Vec<f64> = prs.iter()
        .filter_map(|p| {
            let merged = p.merged_at?;
            Some((merged - p.created_at).num_minutes() as f64 / 60.0)
        })
        .collect();
    if hours.is_empty() {
        return DurationMetric { median_hours: 0.0, level: DoraLevel::NoData, history: vec![] };
    }
    hours.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = median_f64(&hours);
    let level = if median <= 24.0      { DoraLevel::Elite  }
               else if median <= 168.0 { DoraLevel::High   }
               else if median <= 720.0 { DoraLevel::Medium }
               else                    { DoraLevel::Low    };
    DurationMetric { median_hours: median, level, history: hours.iter().copied().take(30).collect() }
}

fn change_failure_rate(
    successes: &[&Deployment],
    failures:  &[&Deployment],
    _since:    DateTime<Utc>,
    _now:      DateTime<Utc>,
) -> RateMetric {
    let total = successes.len() + failures.len();
    let rate  = if total == 0 { 0.0 } else { failures.len() as f64 / total as f64 };
    let level = if rate <= 0.05 { DoraLevel::Elite  }
               else if rate <= 0.10 { DoraLevel::High   }
               else if rate <= 0.15 { DoraLevel::Medium }
               else                 { DoraLevel::Low    };
    RateMetric { rate, level, history: vec![rate] }
}

fn mttr(
    failures:   &[&Deployment],
    recoveries: &[&Deployment],
    _since:     DateTime<Utc>,
    _now:       DateTime<Utc>,
) -> DurationMetric {
    // Pair each failure with the next success deployment in the same repo
    let mut hours: Vec<f64> = vec![];
    for fail in failures {
        if let Some(recovery) = recoveries.iter()
            .filter(|r| r.repo == fail.repo && r.created_at > fail.created_at)
            .min_by_key(|r| r.created_at)
        {
            let h = (recovery.created_at - fail.created_at).num_minutes() as f64 / 60.0;
            hours.push(h);
        }
    }
    hours.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = median_f64(&hours);
    let level = if median <= 1.0       { DoraLevel::Elite  }
               else if median <= 24.0  { DoraLevel::High   }
               else if median <= 168.0 { DoraLevel::Medium }
               else                    { DoraLevel::Low    };
    DurationMetric { median_hours: median, level, history: hours }
}

fn pr_cycle_time(prs: &[&PullRequest]) -> DurationMetric {
    let mut hours: Vec<f64> = prs.iter()
        .filter_map(|p| p.merged_at.map(|m| (m - p.created_at).num_minutes() as f64 / 60.0))
        .collect();
    if hours.is_empty() {
        return DurationMetric { median_hours: 0.0, level: DoraLevel::NoData, history: vec![] };
    }
    hours.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = median_f64(&hours);
    let level = if median <= 24.0      { DoraLevel::Elite  }
               else if median <= 72.0  { DoraLevel::High   }
               else if median <= 240.0 { DoraLevel::Medium }
               else                    { DoraLevel::Low    };
    DurationMetric { median_hours: median, level, history: hours.iter().copied().take(30).collect() }
}

fn review_turnaround(prs: &[&PullRequest]) -> DurationMetric {
    let mut hours: Vec<f64> = prs.iter()
        .filter_map(|p| p.first_review_at.map(|r| (r - p.created_at).num_minutes() as f64 / 60.0))
        .collect();
    if hours.is_empty() {
        return DurationMetric { median_hours: 0.0, level: DoraLevel::NoData, history: vec![] };
    }
    hours.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = median_f64(&hours);
    let level = if median <= 4.0      { DoraLevel::Elite  }
               else if median <= 24.0 { DoraLevel::High   }
               else if median <= 72.0 { DoraLevel::Medium }
               else                   { DoraLevel::Low    };
    DurationMetric { median_hours: median, level, history: hours.iter().copied().take(30).collect() }
}

fn ci_pass_rate(runs: &[&WorkflowRun], since: DateTime<Utc>, now: DateTime<Utc>) -> RateMetric {
    let total  = runs.len();
    if total == 0 {
        return RateMetric { rate: 0.0, level: DoraLevel::NoData, history: vec![] };
    }
    let passed   = runs.iter().filter(|r| r.passed()).count();
    let rate     = passed as f64 / total as f64;
    let level    = if rate >= 0.95 { DoraLevel::Elite  }
                  else if rate >= 0.85 { DoraLevel::High   }
                  else if rate >= 0.70 { DoraLevel::Medium }
                  else                 { DoraLevel::Low    };

    // Daily pass rates for sparkline
    let days = (now - since).num_days() as usize;
    let mut day_pass   = vec![0u32; days.max(1)];
    let mut day_total  = vec![0u32; days.max(1)];
    for r in runs {
        let age = (now - r.created_at).num_days() as usize;
        if age < days {
            let idx = days - 1 - age;
            day_total[idx] += 1;
            if r.passed() { day_pass[idx] += 1; }
        }
    }
    let history = day_pass.iter().zip(day_total.iter())
        .map(|(p, t)| if *t == 0 { 1.0 } else { *p as f64 / *t as f64 })
        .collect();

    RateMetric { rate, level, history }
}

fn median_pr_size(prs: &[&PullRequest]) -> f64 {
    let mut sizes: Vec<f64> = prs.iter()
        .map(|p| (p.additions + p.deletions) as f64)
        .collect();
    sizes.sort_by(|a, b| a.partial_cmp(b).unwrap());
    median_f64(&sizes)
}

fn median_f64(sorted: &[f64]) -> f64 {
    let n = sorted.len();
    if n == 0 { return 0.0; }
    if n % 2 == 0 { (sorted[n/2 - 1] + sorted[n/2]) / 2.0 }
    else           { sorted[n/2] }
}

fn avg_cycle_hours(merged: &[&PullRequest]) -> f64 {
    if merged.is_empty() { return 0.0; }
    let sum: f64 = merged.iter()
        .filter_map(|p| p.merged_at.map(|m| (m - p.created_at).num_minutes() as f64 / 60.0))
        .sum();
    sum / merged.len() as f64
}

fn avg_review_hours(prs: &[&PullRequest]) -> f64 {
    let v: Vec<f64> = prs.iter()
        .filter_map(|p| p.first_review_at.map(|r| (r - p.created_at).num_minutes() as f64 / 60.0))
        .collect();
    if v.is_empty() { return 0.0; }
    v.iter().sum::<f64>() / v.len() as f64
}

fn pass_rate_of(runs: &[&WorkflowRun]) -> f64 {
    if runs.is_empty() { return 1.0; }
    runs.iter().filter(|r| r.passed()).count() as f64 / runs.len() as f64
}
