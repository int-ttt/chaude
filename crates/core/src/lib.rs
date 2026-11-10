//! chaude core: OS helpers, argument parsing and the settings generator, shared by the CLI and the GUI.

pub mod agent_store;
pub mod api;
pub mod budget;
pub mod cache_ttl;
pub mod category;
pub mod child_set;
pub mod clients;
pub mod event;
pub mod forward;
pub mod haiku_pool;
pub mod handoff;
pub mod i18n;
pub mod import_claude;
pub mod import_view;
pub mod lean;
pub mod localtime;
pub mod mcp;
pub mod metrics;
pub mod metrics_report;
pub mod mode;
pub mod orch;
pub mod osenv;
pub mod pick_child;
pub mod proc;
pub mod record;
pub mod record_folder;
pub mod record_turn;
pub mod resident;
pub mod resume_group;
pub mod resume_note;
#[cfg(test)]
mod role_coexist;
pub mod rotate;
pub mod rotate_defer;
pub mod settings;
pub mod turn_spend;
pub mod usage_poll;

#[cfg(test)]
pub(crate) mod testutil {
    use std::path::{Path, PathBuf};

    /// Empty directory under the system temp dir, removed on drop. Unique per process and tag.
    pub struct TempDir(PathBuf);

    impl TempDir {
        pub fn new(tag: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("chaude-core-{}-{tag}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("create temp dir");
            TempDir(dir)
        }

        pub fn path(&self) -> &Path {
            &self.0
        }
    }

    /// Config with the clients and agents files under `dir`, so a test never writes ~/.chaude.
    pub fn isolated(dir: &Path) -> crate::api::Config {
        crate::api::Config {
            agents_file: Some(dir.join("agents.json")),
            clients_file: Some(dir.join("clients.json")),
            ..crate::api::Config::default()
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
