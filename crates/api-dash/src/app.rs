//! Application state and global context

#![allow(dead_code)]

use gpui::Global;
use std::path::PathBuf;

use crate::models::{Environment, RequestTab};

/// Global application state
#[derive(Default)]
pub struct AppState {
    /// Currently open workspace directory
    pub workspace_path: Option<PathBuf>,
    /// Available environments
    pub environments: Vec<Environment>,
    /// Active environment name
    pub active_environment: Option<String>,
    /// Open request tabs
    pub open_tabs: Vec<RequestTab>,
    /// Currently active tab index
    pub active_tab_index: usize,
}

impl Global for AppState {}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the active environment
    pub fn active_env(&self) -> Option<&Environment> {
        self.active_environment
            .as_ref()
            .and_then(|name| self.environments.iter().find(|e| &e.name == name))
    }

    /// Set the active environment by name
    pub fn set_active_environment(&mut self, name: Option<String>) {
        self.active_environment = name;
    }

    /// Add a new tab
    pub fn add_tab(&mut self, tab: RequestTab) {
        self.open_tabs.push(tab);
        self.active_tab_index = self.open_tabs.len() - 1;
    }

    /// Close a tab by index
    pub fn close_tab(&mut self, index: usize) {
        if index < self.open_tabs.len() {
            self.open_tabs.remove(index);
            if self.active_tab_index >= self.open_tabs.len() && !self.open_tabs.is_empty() {
                self.active_tab_index = self.open_tabs.len() - 1;
            }
        }
    }

    /// Get the active tab
    pub fn active_tab(&self) -> Option<&RequestTab> {
        self.open_tabs.get(self.active_tab_index)
    }

    /// Get the active tab mutably
    pub fn active_tab_mut(&mut self) -> Option<&mut RequestTab> {
        self.open_tabs.get_mut(self.active_tab_index)
    }
}
