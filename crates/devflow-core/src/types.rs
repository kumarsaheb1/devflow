use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Config {
    pub token:          String,
    pub owner:          String,
    pub repos:          Vec<String>, // empty = fetch all
    pub lookback_days:  u32,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        use anyhow::Context;
        dotenvy::dotenv().ok();
        let token = std::env::var("GITHUB_TOKEN").context("GITHUB_TOKEN not set")?;
        let owner = std::env::var("GITHUB_OWNER").context("GITHUB_OWNER not set")?;
        let repos = std::env::var("GITHUB_REPOS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        let lookback_days = std::env::var("DEVFLOW_LOOKBACK_DAYS")
            .unwrap_or_else(|_| "90".into())
            .parse()
            .unwrap_or(90);
        Ok(Self { token, owner, repos, lookback_days })
    }
}

// ── Raw GitHub domain types ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub number:          u64,
    pub repo:            String,
    pub title:           String,
    pub author:          String,
    pub created_at:      DateTime<Utc>,
    pub merged_at:       Option<DateTime<Utc>>,
    pub closed_at:       Option<DateTime<Utc>>,
    pub first_review_at: Option<DateTime<Utc>>,
    pub additions:       u32,
    pub deletions:       u32,
    pub changed_files:   u32,
    pub base_ref:        String,
}

impl PullRequest {
    pub fn size_bucket(&self) -> &'static str {
        let lines = self.additions + self.deletions;
        match lines {
            0..=50    => "XS",
            51..=200  => "S",
            201..=500 => "M",
            501..=999 => "L",
            _         => "XL",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deployment {
    pub id:           u64,
    pub repo:         String,
    pub environment:  String,
    pub sha:          String,
    pub created_at:   DateTime<Utc>,
    pub status:       DeploymentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DeploymentStatus {
    Success,
    Failure,
    InProgress,
    Pending,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub id:           u64,
    pub repo:         String,
    pub name:         String,
    pub branch:       String,
    pub conclusion:   Option<String>,
    pub created_at:   DateTime<Utc>,
    pub updated_at:   DateTime<Utc>,
    pub duration_secs: Option<i64>,
}

impl WorkflowRun {
    pub fn passed(&self) -> bool {
        self.conclusion.as_deref() == Some("success")
    }
    pub fn failed(&self) -> bool {
        matches!(self.conclusion.as_deref(), Some("failure") | Some("timed_out"))
    }
}

// ── Computed metrics ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoraMetrics {
    pub deployment_frequency:  FrequencyMetric,
    pub lead_time:             DurationMetric,
    pub change_failure_rate:   RateMetric,
    pub mean_time_to_recovery: DurationMetric,

    // Bonus
    pub pr_cycle_time:         DurationMetric,
    pub review_turnaround:     DurationMetric,
    pub ci_pass_rate:          RateMetric,
    pub median_pr_size:        f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FrequencyMetric {
    /// Deployments per day averaged over the window
    pub per_day:  f64,
    /// DORA level: Elite / High / Medium / Low
    pub level:    DoraLevel,
    /// Historical daily counts for sparkline (oldest → newest)
    pub history:  Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DurationMetric {
    /// Median duration in hours
    pub median_hours: f64,
    pub level:        DoraLevel,
    /// Historical daily medians (hours)
    pub history:      Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RateMetric {
    /// 0.0 – 1.0
    pub rate:    f64,
    pub level:   DoraLevel,
    /// Historical daily rates
    pub history: Vec<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DoraLevel {
    Elite,
    High,
    #[default]
    Medium,
    Low,
    /// Not enough data to compute a level
    NoData,
}

impl DoraLevel {
    pub fn label(&self) -> &'static str {
        match self {
            DoraLevel::Elite  => "Elite",
            DoraLevel::High   => "High",
            DoraLevel::Medium => "Medium",
            DoraLevel::Low    => "Low",
            DoraLevel::NoData => "No data",
        }
    }
    pub fn color_hint(&self) -> &'static str {
        match self {
            DoraLevel::Elite  => "green",
            DoraLevel::High   => "cyan",
            DoraLevel::Medium => "yellow",
            DoraLevel::Low    => "red",
            DoraLevel::NoData => "gray",
        }
    }
}


// ── Per-repo breakdown ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoStats {
    pub repo:              String,
    pub pr_count:          usize,
    pub merged_count:      usize,
    pub avg_cycle_hours:   f64,
    pub avg_review_hours:  f64,
    pub ci_pass_rate:      f64,
    pub deploy_count:      usize,
}

// ── Per-author breakdown ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorStats {
    pub author:            String,
    pub pr_count:          usize,
    pub merged_count:      usize,
    pub avg_cycle_hours:   f64,
    pub avg_pr_size:       f64,
    pub reviews_given:     usize,
}

// ── Full dataset ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Dataset {
    pub owner:       String,
    pub since:       Option<DateTime<Utc>>,
    pub until:       Option<DateTime<Utc>>,
    pub prs:         Vec<PullRequest>,
    pub deployments: Vec<Deployment>,
    pub ci_runs:     Vec<WorkflowRun>,
}
