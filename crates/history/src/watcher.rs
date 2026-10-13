use crate::adapter::AgentAdapter;
use crate::adapters::default_adapters;
use crate::compactor::CompactorWorker;
use crate::db::HistoryDb;
use pfmt_core::Config;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub struct WatcherDaemon {
    db: HistoryDb,
    config: Config,
    adapters: Vec<Box<dyn AgentAdapter>>,
    compactor: CompactorWorker,
    running: Arc<AtomicBool>,
}

impl WatcherDaemon {
    pub fn new(db: HistoryDb, config: Config) -> Self {
        let compactor = CompactorWorker::new(db.clone(), &config);
        let adapters = default_adapters();
        Self {
            db,
            config,
            adapters,
            compactor,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn with_adapters(
        db: HistoryDb,
        config: Config,
        adapters: Vec<Box<dyn AgentAdapter>>,
    ) -> Self {
        let compactor = CompactorWorker::new(db.clone(), &config);
        Self {
            db,
            config,
            adapters,
            compactor,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Single scan cycle: inspects running agents, upserts sessions on prompt,
    /// enqueues compactor jobs on finished assistant messages, and processes pending jobs.
    pub async fn run_once(&self) -> usize {
        let mut sessions_updated = 0;

        for adapter in &self.adapters {
            let agent_name = adapter.agent_name();
            // Check if agent is enabled in config
            let app_slug = agent_name.to_lowercase().replace(' ', "");
            if !self.config.is_stage_enabled(&app_slug, "history") {
                continue;
            }

            let sessions = adapter.scan_sessions();
            for session in sessions {
                if !session.has_first_prompt {
                    continue;
                }

                // Upsert session into SQLite
                if let Ok(session_pk) = self.db.upsert_session(
                    agent_name,
                    &session.session_id,
                    Some(&session.front_app),
                    session.model.as_deref(),
                    session.project_path.as_deref(),
                    Some(&session.source_ref),
                    Some(&session.source_cursor),
                ) {
                    sessions_updated += 1;

                    // Only enqueue compact job when source_cursor changed since last summary
                    if session.assistant_message_finished && !session.turns.is_empty() {
                        // Simple backoff: only enqueue if cursor changed (would need DB read; simplified here)
                        let _ = self.db.enqueue_compact_job(session_pk);
                    }
                }
            }
        }

        // Process any queued compact job
        while let Ok(Some(job)) = self.db.claim_next_compact_job() {
            // Find turns for this session from adapter
            let mut turns = Vec::new();
            for adapter in &self.adapters {
                if adapter.agent_name() == job.agent {
                    let sessions = adapter.scan_sessions();
                    if let Some(s) = sessions
                        .into_iter()
                        .find(|s| s.session_id == job.session_id)
                    {
                        turns = s.turns;
                        break;
                    }
                }
            }

            let _ = self.compactor.process_job(&job, &turns).await;
        }

        sessions_updated
    }

    /// Runs the continuous background watcher loop with a 1-second interval.
    pub async fn start(&self, shutdown_signal: tokio::sync::watch::Receiver<bool>) {
        self.running.store(true, Ordering::SeqCst);
        println!("🚀 pfmt watch daemon started. Monitoring agent sessions...");

        let mut interval = tokio::time::interval(Duration::from_millis(1000));
        let mut shutdown = shutdown_signal;

        while self.running.load(Ordering::SeqCst) {
            tokio::select! {
                _ = interval.tick() => {
                    self.run_once().await;
                }
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        println!("🛑 Shutdown signal received. Stopping pfmt watch...");
                        self.running.store(false, Ordering::SeqCst);
                        break;
                    }
                }
            }
        }
    }
}
