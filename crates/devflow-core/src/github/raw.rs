use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RawPullRequest {
    pub number:     u64,
    pub title:      String,
    pub user:       RawUser,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub merged_at:  Option<DateTime<Utc>>,
    pub closed_at:  Option<DateTime<Utc>>,
    pub base:       RawRef,
}

#[derive(Debug, Deserialize, Default)]
pub struct RawPrDetail {
    pub additions:     u32,
    pub deletions:     u32,
    pub changed_files: u32,
}

#[derive(Debug, Deserialize)]
pub struct RawUser { pub login: String }

#[derive(Debug, Deserialize)]
pub struct RawRef {
    #[serde(rename = "ref")]
    pub ref_name: String,
}

#[derive(Debug, Deserialize)]
pub struct RawDeployment {
    pub id:          u64,
    pub sha:         String,
    pub environment: String,
    pub created_at:  DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct RawDeploymentStatus { pub state: String }

#[derive(Debug, Deserialize)]
pub struct RawWorkflowRunsPage {
    pub workflow_runs: Vec<RawWorkflowRun>,
}

#[derive(Debug, Deserialize)]
pub struct RawWorkflowRun {
    pub id:          u64,
    pub name:        String,
    pub head_branch: String,
    pub conclusion:  Option<String>,
    pub created_at:  DateTime<Utc>,
    pub updated_at:  DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct RawRepo {
    pub name:      String,
    pub pushed_at: Option<chrono::DateTime<chrono::Utc>>,
}
