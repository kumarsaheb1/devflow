use tokio::sync::mpsc;

use devflow_core::{
    metrics,
    types::{AuthorStats, Dataset, DoraMetrics, RepoStats},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tab {
    Overview,
    Repos,
    Authors,
    CI,
}

impl Tab {
    pub const ALL: &'static [Tab] = &[Tab::Overview, Tab::Repos, Tab::Authors, Tab::CI];
    pub fn title(&self) -> &'static str {
        match self {
            Tab::Overview => "Overview",
            Tab::Repos    => "Repos",
            Tab::Authors  => "Authors",
            Tab::CI       => "CI",
        }
    }
}

#[derive(Debug, Clone)]
pub enum LoadState {
    Loading,
    Loaded,
    Error(String),
}

pub struct App {
    pub demo:       bool,
    pub load_state: LoadState,
    pub tab:        usize,
    pub selected:   usize,

    pub dataset:    Option<Dataset>,
    pub metrics:    Option<DoraMetrics>,
    pub repo_stats: Vec<RepoStats>,
    pub author_stats: Vec<AuthorStats>,
    pub lookback_days: u32,

    pub fetch_tx:   mpsc::UnboundedSender<anyhow::Result<Dataset>>,
    pub fetch_rx:   mpsc::UnboundedReceiver<anyhow::Result<Dataset>>,
}

impl App {
    pub fn new(demo: bool) -> Self {
        let (fetch_tx, fetch_rx) = mpsc::unbounded_channel();
        Self {
            demo,
            load_state: LoadState::Loading,
            tab:        0,
            selected:   0,
            dataset:    None,
            metrics:    None,
            repo_stats: vec![],
            author_stats: vec![],
            lookback_days: 90,
            fetch_tx,
            fetch_rx,
        }
    }

    pub fn on_data_loaded(&mut self, ds: Dataset) {
        self.metrics      = Some(metrics::calculate(&ds, self.lookback_days));
        self.repo_stats   = metrics::per_repo(&ds, self.lookback_days);
        self.author_stats = metrics::per_author(&ds, self.lookback_days);
        self.dataset      = Some(ds);
        self.load_state   = LoadState::Loaded;
    }

    pub fn next_tab(&mut self) { self.tab = (self.tab + 1) % Tab::ALL.len(); self.selected = 0; }
    pub fn prev_tab(&mut self) { self.tab = (self.tab + Tab::ALL.len() - 1) % Tab::ALL.len(); self.selected = 0; }

    pub fn move_down(&mut self) {
        let max = self.current_list_len().saturating_sub(1);
        self.selected = (self.selected + 1).min(max);
    }
    pub fn move_up(&mut self) { self.selected = self.selected.saturating_sub(1); }

    pub fn refresh(&mut self) {
        self.load_state = LoadState::Loading;
        self.metrics    = None;
    }

    fn current_list_len(&self) -> usize {
        match Tab::ALL.get(self.tab) {
            Some(Tab::Repos)   => self.repo_stats.len(),
            Some(Tab::Authors) => self.author_stats.len(),
            Some(Tab::CI)      => self.dataset.as_ref().map(|d| d.ci_runs.len()).unwrap_or(0),
            _ => 0,
        }
    }
}
