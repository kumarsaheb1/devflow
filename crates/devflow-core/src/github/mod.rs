mod raw;

use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use futures_util::stream::{self, StreamExt};
use reqwest::{header, Client};
use tracing::{info, warn};

use crate::types::*;
use raw::*;

/// Max repos fetched concurrently in `fetch_all`.
///
/// Each repo fans out its own concurrent PR-detail / deployment-status
/// lookups (see `DETAIL_CONCURRENCY`), so the *actual* number of
/// simultaneous requests to the GitHub API is roughly
/// `REPO_CONCURRENCY * (2 * DETAIL_CONCURRENCY)`. Keep both constants
/// small - GitHub's secondary/abuse-detection rate limit triggers on
/// request bursts, not just total-requests-per-hour, and once tripped it
/// blocks requests (as if the primary limit were exhausted) independently
/// of the real quota reported by `/rate_limit`.
const REPO_CONCURRENCY: usize = 3;

/// Max concurrent in-flight requests for a single repo's per-item detail
/// lookups (PR details, deployment statuses). See `REPO_CONCURRENCY`.
const DETAIL_CONCURRENCY: usize = 3;

pub struct GithubClient {
    http:  Client,
    owner: String,
    token: String,
}

impl GithubClient {
    pub fn new(cfg: &Config) -> anyhow::Result<Self> {
        let mut headers = header::HeaderMap::new();
        headers.insert(header::ACCEPT, "application/vnd.github+json".parse()?);
        headers.insert("X-GitHub-Api-Version", "2022-11-28".parse()?);
        let http = Client::builder()
            .user_agent("devflow/0.1")
            .default_headers(headers)
            .build()?;
        Ok(Self { http, owner: cfg.owner.clone(), token: cfg.token.clone() })
    }

    fn auth(&self) -> String { format!("Bearer {}", self.token) }

    // ── Public fetch methods ──────────────────────────────────────────────────

    pub async fn fetch_all(&self, cfg: &Config) -> anyhow::Result<Dataset> {
        let since = Utc::now() - Duration::days(cfg.lookback_days as i64);

        let repos = if cfg.repos.is_empty() {
            info!("fetching repo list for {}", self.owner);
            let mut all = self.list_repos().await?;
            // Keep only the 50 most recently pushed repos to avoid scanning dead repos
            all.truncate(50);
            all
        } else {
            cfg.repos.clone()
        };

        info!("analysing {} repos since {}", repos.len(), since.date_naive());

        let mut ds = Dataset {
            owner: self.owner.clone(),
            since: Some(since),
            until: Some(Utc::now()),
            ..Default::default()
        };

        // Fetch repos concurrently in batches to avoid rate-limiting.
        //
        // NB: iterate over owned `String`s (not `&String`) here. Using
        // `repos.iter()` makes the `.map()` closure take a borrowed item,
        // which forces rustc to infer a higher-ranked `for<'a> Fn(&'a
        // String) -> Future` signature for it. Since the returned future
        // also captures `self` with a fixed (non-higher-ranked) lifetime,
        // that inference fails with "implementation of `FnOnce` is not
        // general enough" - surfaced lazily at the `tokio::spawn` call site
        // where the future is required to be fully concrete/'static.
        //
        // The per-repo timeout is generous (60s) because `fetch_repo_data`
        // itself now fans out its PR-detail / deployment-status lookups
        // concurrently (see `list_merged_prs` / `list_deployments` below),
        // so busy repos should comfortably finish well inside it.
        let results: Vec<_> = stream::iter(repos.clone())
            .map(|repo| {
                let since = since;
                async move {
                    let r = tokio::time::timeout(
                        std::time::Duration::from_secs(60),
                        self.fetch_repo_data(&repo, since),
                    ).await;
                    (repo, r)
                }
            })
            .buffer_unordered(REPO_CONCURRENCY)
            .collect()
            .await;

        for (repo, result) in results {
            match result {
                Ok(Ok((prs, deploys, runs))) => {
                    info!("  {repo}: {} PRs, {} deploys, {} CI runs", prs.len(), deploys.len(), runs.len());
                    ds.prs.extend(prs);
                    ds.deployments.extend(deploys);
                    ds.ci_runs.extend(runs);
                }
                Ok(Err(e))  => warn!("  {repo} error: {e}"),
                Err(_)      => warn!("  {repo} timed out"),
            }
        }

        info!("TOTAL: {} PRs, {} deployments, {} CI runs",
            ds.prs.len(), ds.deployments.len(), ds.ci_runs.len());
        Ok(ds)
    }

    async fn fetch_repo_data(
        &self,
        repo:  &str,
        since: DateTime<Utc>,
    ) -> anyhow::Result<(Vec<PullRequest>, Vec<Deployment>, Vec<WorkflowRun>)> {
        let (prs, deploys, runs) = tokio::try_join!(
            self.list_merged_prs(repo, since),
            self.list_deployments(repo, since),
            self.list_workflow_runs(repo, since),
        )?;
        Ok((prs, deploys, runs))
    }

    // ── PRs ───────────────────────────────────────────────────────────────────

    async fn list_merged_prs(
        &self,
        repo:  &str,
        since: DateTime<Utc>,
    ) -> anyhow::Result<Vec<PullRequest>> {
        let mut merged = vec![];
        let mut page = 1u32;

        loop {
            let url = format!(
                "https://api.github.com/repos/{}/{}/pulls\
                 ?state=closed&sort=updated&direction=desc&per_page=100&page={page}",
                self.owner, repo
            );
            let raw: Vec<RawPullRequest> = self
                .http.get(&url)
                .header(header::AUTHORIZATION, self.auth())
                .send().await?
                .error_for_status()?
                .json().await?;

            if raw.is_empty() { break; }

            let mut stop = false;
            for r in raw {
                if r.updated_at < since { stop = true; break; }
                if r.merged_at.is_none() { continue; }
                merged.push(r);
            }
            if stop { break; }
            page += 1;
            if page > 10 { break; } // cap at 1000 PRs
        }

        // Fetch each PR's additions/deletions/changed_files concurrently
        // instead of one-at-a-time - repos with lots of merged PRs were
        // easily blowing past the per-repo fetch timeout otherwise.
        let numbers: Vec<u64> = merged.iter().map(|r| r.number).collect();
        let details: HashMap<u64, (u32, u32, u32)> = stream::iter(numbers)
            .map(|number| async move {
                let detail = self.pr_detail(repo, number).await.unwrap_or_default();
                (number, detail)
            })
            .buffer_unordered(DETAIL_CONCURRENCY)
            .collect()
            .await;

        let prs = merged.into_iter().map(|r| {
            let detail = details.get(&r.number).copied().unwrap_or_default();
            PullRequest {
                number:          r.number,
                repo:            repo.to_owned(),
                title:           r.title,
                author:          r.user.login,
                created_at:      r.created_at,
                merged_at:       r.merged_at,
                closed_at:       r.closed_at,
                first_review_at: None, // enriched separately if needed
                additions:       detail.0,
                deletions:       detail.1,
                changed_files:   detail.2,
                base_ref:        r.base.ref_name,
            }
        }).collect();

        Ok(prs)
    }

    async fn pr_detail(&self, repo: &str, number: u64) -> anyhow::Result<(u32, u32, u32)> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/pulls/{}", self.owner, repo, number
        );
        let r: RawPrDetail = self.http.get(&url)
            .header(header::AUTHORIZATION, self.auth())
            .send().await?
            .error_for_status()?
            .json().await?;
        Ok((r.additions, r.deletions, r.changed_files))
    }

    // ── Deployments ───────────────────────────────────────────────────────────

    async fn list_deployments(
        &self,
        repo:  &str,
        since: DateTime<Utc>,
    ) -> anyhow::Result<Vec<Deployment>> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/deployments?per_page=100",
            self.owner, repo
        );
        let raw: Vec<RawDeployment> = self.http.get(&url)
            .header(header::AUTHORIZATION, self.auth())
            .send().await?
            .error_for_status()?
            .json().await?;

        // The API returns deployments newest-first, so stop as soon as we
        // cross the `since` boundary.
        let recent: Vec<RawDeployment> = raw.into_iter()
            .take_while(|r| r.created_at >= since)
            .collect();

        // Fetch each deployment's latest status concurrently rather than
        // one-at-a-time - repos with lots of deployments were easily
        // blowing past the per-repo fetch timeout otherwise.
        let ids: Vec<u64> = recent.iter().map(|r| r.id).collect();
        let statuses: HashMap<u64, DeploymentStatus> = stream::iter(ids)
            .map(|id| async move {
                let status = self.deployment_status(repo, id).await.unwrap_or(DeploymentStatus::Other);
                (id, status)
            })
            .buffer_unordered(DETAIL_CONCURRENCY)
            .collect()
            .await;

        let result = recent.into_iter().map(|r| {
            let status = statuses.get(&r.id).cloned().unwrap_or(DeploymentStatus::Pending);
            Deployment {
                id:          r.id,
                repo:        repo.to_owned(),
                environment: r.environment,
                sha:         r.sha,
                created_at:  r.created_at,
                status,
            }
        }).collect();

        Ok(result)
    }

    async fn deployment_status(&self, repo: &str, id: u64) -> anyhow::Result<DeploymentStatus> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/deployments/{}/statuses?per_page=1",
            self.owner, repo, id
        );
        let raw: Vec<RawDeploymentStatus> = self.http.get(&url)
            .header(header::AUTHORIZATION, self.auth())
            .send().await?
            .error_for_status()?
            .json().await?;
        Ok(raw.first().map(|s| match s.state.as_str() {
            "success"     => DeploymentStatus::Success,
            "failure" | "error" => DeploymentStatus::Failure,
            "in_progress" => DeploymentStatus::InProgress,
            "pending"     => DeploymentStatus::Pending,
            _             => DeploymentStatus::Other,
        }).unwrap_or(DeploymentStatus::Pending))
    }

    // ── CI / Workflow runs ────────────────────────────────────────────────────

    async fn list_workflow_runs(
        &self,
        repo:  &str,
        since: DateTime<Utc>,
    ) -> anyhow::Result<Vec<WorkflowRun>> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/actions/runs\
             ?per_page=100&branch=main&status=completed",
            self.owner, repo
        );
        let resp: RawWorkflowRunsPage = self.http.get(&url)
            .header(header::AUTHORIZATION, self.auth())
            .send().await?
            .error_for_status()?
            .json().await?;

        let runs = resp.workflow_runs.into_iter()
            .filter(|r| r.created_at >= since)
            .map(|r| {
                let duration = r.updated_at.signed_duration_since(r.created_at).num_seconds();
                WorkflowRun {
                    id:            r.id,
                    repo:          repo.to_owned(),
                    name:          r.name,
                    branch:        r.head_branch,
                    conclusion:    r.conclusion,
                    created_at:    r.created_at,
                    updated_at:    r.updated_at,
                    duration_secs: Some(duration),
                }
            })
            .collect();
        Ok(runs)
    }

    // ── Repos ─────────────────────────────────────────────────────────────────

    async fn list_repos(&self) -> anyhow::Result<Vec<String>> {
        let url = format!(
            "https://api.github.com/orgs/{}/repos?per_page=100&type=all&sort=pushed",
            self.owner
        );
        let resp = self.http.get(&url)
            .header(header::AUTHORIZATION, self.auth())
            .send().await?;

        if resp.status() == 404 {
            let url2 = format!(
                "https://api.github.com/users/{}/repos?per_page=100&sort=pushed",
                self.owner
            );
            let raw: Vec<RawRepo> = self.http.get(&url2)
                .header(header::AUTHORIZATION, self.auth())
                .send().await?.error_for_status()?.json().await?;
            return Ok(raw.into_iter().map(|r| r.name).collect());
        }
        let mut raw: Vec<RawRepo> = resp.error_for_status()?.json().await?;
        // Sort by most recently pushed so truncate(50) keeps active repos
        raw.sort_by(|a, b| b.pushed_at.cmp(&a.pushed_at));
        Ok(raw.into_iter().map(|r| r.name).collect())
    }
}
