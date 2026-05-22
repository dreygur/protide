use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use gpui::Context;
use protide_core::collection_runner::{RunConfig, RunProgress};

pub mod render;

#[derive(Clone, Debug, PartialEq)]
pub enum RowStatus {
    Pending,
    Running,
    Passed,
    Failed(String),
}

#[derive(Clone, Debug)]
pub struct RunnerRow {
    pub name: String,
    pub status: RowStatus,
}

pub struct RunnerPanel {
    pub(super) rows: Vec<RunnerRow>,
    pub(super) running: bool,
    pub(super) current: usize,
    pub(super) total: usize,
    pub(super) stop_flag: Arc<AtomicBool>,
}

impl RunnerPanel {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let _ = cx;
        Self {
            rows: Vec::new(),
            running: false,
            current: 0,
            total: 0,
            stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(
        &mut self,
        collection_path: PathBuf,
        env_vars: HashMap<String, String>,
        cx: &mut Context<Self>,
    ) {
        let config = RunConfig {
            collection_path,
            env_vars,
            stop_on_failure: false,
        };

        let (progress_tx, progress_rx) = async_channel::unbounded::<RunProgress>();
        let stop_flag = Arc::new(AtomicBool::new(false));

        self.rows.clear();
        self.running = true;
        self.current = 0;
        self.total = 0;
        self.stop_flag = stop_flag.clone();
        cx.notify();

        std::thread::spawn(move || {
            protide_core::collection_runner::run_collection(config, progress_tx);
        });

        cx.spawn(async move |panel, cx| {
            while let Ok(event) = progress_rx.recv().await {
                if stop_flag.load(Ordering::Relaxed) {
                    break;
                }
                let done = matches!(event, RunProgress::Done);
                panel.update(cx, |this, cx| {
                    match event {
                        RunProgress::Starting { index, total, name } => {
                            this.total = total;
                            this.current = index;
                            while this.rows.len() <= index {
                                this.rows.push(RunnerRow {
                                    name: String::new(),
                                    status: RowStatus::Pending,
                                });
                            }
                            this.rows[index] = RunnerRow { name, status: RowStatus::Running };
                            cx.notify();
                        }
                        RunProgress::Completed { index, result } => {
                            if let Some(row) = this.rows.get_mut(index) {
                                row.status = match result.result {
                                    Ok(_) => RowStatus::Passed,
                                    Err(e) => RowStatus::Failed(e),
                                };
                            }
                            cx.notify();
                        }
                        RunProgress::Done => {
                            this.running = false;
                            cx.notify();
                        }
                    }
                }).ok();
                if done { break; }
            }
        }).detach();
    }

    pub fn stop(&mut self, cx: &mut Context<Self>) {
        self.stop_flag.store(true, Ordering::Relaxed);
        self.running = false;
        cx.notify();
    }

    pub fn passed(&self) -> usize {
        self.rows.iter().filter(|r| r.status == RowStatus::Passed).count()
    }

    pub fn failed(&self) -> usize {
        self.rows.iter().filter(|r| matches!(r.status, RowStatus::Failed(_))).count()
    }
}
