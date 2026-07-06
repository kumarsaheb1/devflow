use chrono::{Duration, Utc};
use devflow_core::types::*;

pub fn make_dataset() -> Dataset {
    let now = Utc::now();

    let mut prs = vec![];
    // Generate realistic PR data across 4 repos
    let repo_data: &[(&str, &[(&str, i64, i64, u32, u32)])] = &[
        ("api-service", &[
            ("alice",  2,  26, 120, 45), ("bob",    5,  48, 300, 80),
            ("carol",  1,  12,  40, 15), ("alice",  3,  36, 200, 60),
            ("dave",  10, 168, 800,210), ("bob",    4,  24, 180, 50),
            ("eve",    2,  18,  90, 30), ("alice",  6,  72, 400,120),
            ("carol",  1,   8,  20, 10), ("frank",  3,  30, 150, 45),
            ("dave",   7,  96, 550,160), ("bob",    2,  20, 100, 35),
        ]),
        ("frontend", &[
            ("carol",  1,  10,  80, 20), ("frank",  3,  32, 220, 55),
            ("alice",  2,  22, 130, 40), ("dave",   5,  56, 380,100),
            ("bob",    1,   9,  60, 18), ("carol",  4,  44, 290, 75),
            ("eve",    2,  17,  95, 28), ("frank",  6,  80, 500,140),
            ("alice",  1,  11,  70, 22), ("dave",   3,  34, 200, 58),
        ]),
        ("infra", &[
            ("eve",    8, 120, 600,180), ("alice",  2,  28, 150, 45),
            ("bob",    5,  64, 420,110), ("carol",  1,  16,  80, 25),
            ("eve",    3,  38, 230, 68), ("frank",  4,  48, 310, 85),
        ]),
        ("auth-service", &[
            ("alice",  2,  22, 110, 38), ("bob",    3,  34, 200, 60),
            ("carol",  1,  12,  55, 18), ("dave",   6,  88, 480,140),
            ("alice",  4,  46, 280, 82), ("eve",    2,  19,  90, 28),
        ]),
    ];

    let mut idx = 0u64;
    for (repo, entries) in repo_data {
        for (i, (author, days_ago, cycle_hours, add, del)) in entries.iter().enumerate() {
            let created = now - Duration::days(*days_ago) - Duration::hours(i as i64 * 3);
            let merged  = created + Duration::hours(*cycle_hours);
            let first_review = created + Duration::hours(cycle_hours / 4);
            prs.push(PullRequest {
                number:          100 + idx,
                repo:            repo.to_string(),
                title:           format!("PR #{} in {}", 100 + idx, repo),
                author:          author.to_string(),
                created_at:      created,
                merged_at:       Some(merged),
                closed_at:       Some(merged),
                first_review_at: Some(first_review),
                additions:       *add,
                deletions:       *del,
                changed_files:   (*add / 50 + 1) as u32,
                base_ref:        "main".into(),
            });
            idx += 1;
        }
    }

    // Deployments — prod deploys
    let mut deployments = vec![];
    for (i, (repo, success)) in [
        ("api-service", true), ("api-service", true), ("api-service", false),
        ("api-service", true), ("api-service", true),
        ("frontend",    true), ("frontend",    true), ("frontend",    true),
        ("infra",       true), ("infra",       false), ("infra",      true),
        ("auth-service",true), ("auth-service",true),
    ].iter().enumerate() {
        let days_ago = (i as i64) * 5 + 1;
        deployments.push(Deployment {
            id:          i as u64 + 1,
            repo:        repo.to_string(),
            environment: "production".into(),
            sha:         format!("abc{i:04x}"),
            created_at:  now - Duration::days(days_ago),
            status:      if *success { DeploymentStatus::Success } else { DeploymentStatus::Failure },
        });
    }

    // CI runs — main branch
    let mut ci_runs = vec![];
    let ci_data: &[(&str, &str, &[bool])] = &[
        ("api-service",  "CI",              &[true, true, true, true, true, true, false, true, true, true]),
        ("frontend",     "Build & Test",    &[true, true, true, true, false, true, true, true, true, true]),
        ("infra",        "Terraform Plan",  &[true, true, true, false, true, true, true, true, true, true]),
        ("auth-service", "Security & Test", &[true, true, true, true, true, false, true, true, true, true]),
    ];
    for (r, name, results) in ci_data {
        for (i, passed) in results.iter().enumerate() {
            let created = now - Duration::days(i as i64 * 7);
            ci_runs.push(WorkflowRun {
                id:            ci_runs.len() as u64 + 1,
                repo:          r.to_string(),
                name:          name.to_string(),
                branch:        "main".into(),
                conclusion:    Some(if *passed { "success" } else { "failure" }.into()),
                created_at:    created,
                updated_at:    created + Duration::minutes(8),
                duration_secs: Some(480),
            });
        }
    }

    Dataset {
        owner: "acme".into(),
        since: Some(now - Duration::days(90)),
        until: Some(now),
        prs,
        deployments,
        ci_runs,
    }
}
