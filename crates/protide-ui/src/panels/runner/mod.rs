use gpui::Context;
use protide_core::collection_runner::{RunConfig, RunProgress};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

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
                panel
                    .update(cx, |this, cx| match event {
                        RunProgress::Starting { index, total, name } => {
                            this.total = total;
                            this.current = index;
                            while this.rows.len() <= index {
                                this.rows.push(RunnerRow {
                                    name: String::new(),
                                    status: RowStatus::Pending,
                                });
                            }
                            this.rows[index] = RunnerRow {
                                name,
                                status: RowStatus::Running,
                            };
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
                    })
                    .ok();
                if done {
                    break;
                }
            }
        })
        .detach();
    }

    pub fn stop(&mut self, cx: &mut Context<Self>) {
        self.stop_flag.store(true, Ordering::Relaxed);
        self.running = false;
        cx.notify();
    }

    pub fn passed(&self) -> usize {
        self.rows
            .iter()
            .filter(|r| r.status == RowStatus::Passed)
            .count()
    }

    pub fn failed(&self) -> usize {
        self.rows
            .iter()
            .filter(|r| matches!(r.status, RowStatus::Failed(_)))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext as _, TestAppContext};

    fn row(name: &str, status: RowStatus) -> RunnerRow {
        RunnerRow {
            name: name.to_string(),
            status,
        }
    }

    #[gpui::test]
    async fn a_new_runner_is_idle_and_empty(cx: &mut TestAppContext) {
        let panel = cx.new(RunnerPanel::new);
        panel.read_with(cx, |p, _| {
            assert!(!p.running);
            assert!(p.rows.is_empty());
            assert_eq!((p.current, p.total), (0, 0));
            assert_eq!((p.passed(), p.failed()), (0, 0));
        });
    }

    #[gpui::test]
    async fn the_tallies_count_only_finished_rows(cx: &mut TestAppContext) {
        let panel = cx.new(RunnerPanel::new);
        panel.update(cx, |p, _| {
            p.rows = vec![
                row("a", RowStatus::Passed),
                row("b", RowStatus::Failed("assertion failed".into())),
                row("c", RowStatus::Passed),
                row("d", RowStatus::Running),
                row("e", RowStatus::Pending),
            ];
        });
        panel.read_with(cx, |p, _| {
            assert_eq!(p.passed(), 2);
            assert_eq!(p.failed(), 1);
            assert_eq!(
                p.passed() + p.failed(),
                3,
                "in-flight and queued rows must not be counted as results"
            );
        });
    }

    #[gpui::test]
    async fn a_failure_is_counted_whatever_its_message(cx: &mut TestAppContext) {
        let panel = cx.new(RunnerPanel::new);
        panel.update(cx, |p, _| {
            p.rows = vec![
                row("a", RowStatus::Failed(String::new())),
                row("b", RowStatus::Failed("日本語のエラー".into())),
            ];
        });
        panel.read_with(cx, |p, _| assert_eq!(p.failed(), 2));
    }

    #[gpui::test]
    async fn stopping_marks_the_run_finished_and_raises_the_stop_flag(cx: &mut TestAppContext) {
        let panel = cx.new(RunnerPanel::new);
        panel.update(cx, |p, cx| {
            p.running = true;
            p.stop(cx);
            assert!(!p.running);
            assert!(p.stop_flag.load(Ordering::Relaxed));
        });
    }

    // NOT COVERED: `start()` spawns an OS thread that sends progress over an
    // async channel, waking the gpui task from a foreign thread. gpui's test
    // scheduler treats that as non-determinism and aborts the whole test
    // binary, so the reset-on-start and fresh-stop-flag behaviour cannot be
    // driven from a #[gpui::test] as `start` is currently written - it would
    // need the progress channel injected rather than created inside.
}
