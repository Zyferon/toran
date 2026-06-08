//! Policy loader: reads YAML files from a directory, watches for
//! changes, and exposes an `ArcSwap`-style atomic handle to the
//! latest compiled policy.

use super::compiler::compile_policy;
use super::schema::{CompiledPolicy, PolicyFile};
use anyhow::{Context, Result};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A handle to the live policy set. Internally an `Arc<RwLock<...>>`
/// is fine because the compiled policy is read-heavy and only swapped
/// on (rare) file changes. We hold the lock for microseconds on read
/// and milliseconds on swap.
pub struct PolicyStore {
    dir: PathBuf,
    /// Compiled policies, sorted by (priority desc, name asc) at
    /// reload time. We use a Vec (not a BTreeMap) so that the order
    /// is preserved.
    inner: RwLock<Vec<CompiledPolicy>>,
    default: RwLock<Action>,
    watcher: Option<RecommendedWatcher>,
}

// We need to refer to `Action` directly; re-export it for convenience.
pub use super::schema::Action;

impl PolicyStore {
    /// Load every `*.yaml` / `*.yml` file in `dir` and return a store.
    /// Also spawns a background thread that hot-reloads on file change.
    pub fn load(dir: &Path) -> Result<Arc<Self>> {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("create policy dir {}", dir.display()))?;
        let store = Arc::new(Self {
            dir: dir.to_path_buf(),
            inner: RwLock::new(Vec::new()),
            default: RwLock::new(Action::Allow),
            watcher: None,
        });
        store.reload()?;
        let weak = Arc::downgrade(&store);
        let dir_buf = dir.to_path_buf();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            let Ok(event) = res else { return };
            if matches!(
                event.kind,
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
            ) {
                if let Some(strong) = weak.upgrade() {
                    if let Err(e) = strong.reload() {
                        tracing::warn!(error = %e, "policy hot-reload failed");
                    }
                }
                tracing::debug!(?dir_buf, "policy reload trigger");
            }
        })?;
        watcher.watch(dir, RecursiveMode::NonRecursive)?;
        // Store the watcher by replacing via interior mutability.
        // We use `parking_lot::Mutex` on the watcher itself.
        let watcher_slot: parking_lot::Mutex<Option<RecommendedWatcher>> =
            parking_lot::Mutex::new(Some(watcher));
        // SAFETY: this is a one-shot init. The Mutex lives in the Arc.
        // We attach it via an unsafe write to a `OnceCell`. In practice
        // we leak the Mutex (one allocation, no leak per reload).
        let leaked: &'static parking_lot::Mutex<Option<RecommendedWatcher>> =
            Box::leak(Box::new(watcher_slot));
        let _ = leaked; // we just need it to keep the watcher alive
        // Apply same trick to the store: we cannot mutate `watcher` now
        // because it is behind `Arc`. Skip the field; the watcher lives
        // in the leaked Mutex above.
        let _ = &store.watcher; // suppress unused
        Ok(store)
    }

    /// Re-scan the policy directory and rebuild the in-memory list.
    pub fn reload(&self) -> Result<()> {
        let mut loaded: Vec<(CompiledPolicy, Option<Action>)> = Vec::new();
        for entry in walkdir::WalkDir::new(&self.dir)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if !matches!(ext, "yaml" | "yml") {
                continue;
            }
            match self.load_one(path) {
                Ok((policy, default)) => loaded.push((policy, default)),
                Err(e) => {
                    tracing::error!(file = %path.display(), error = %e, "failed to load policy file");
                }
            }
        }
        // Sort by priority desc, then name asc. Higher-priority
        // policies are evaluated first; ties break by name.
        loaded.sort_by(|a, b| {
            b.0.priority
                .cmp(&a.0.priority)
                .then_with(|| a.0.name.cmp(&b.0.name))
        });
        let mut new_default = Action::Allow;
        let mut new_list: Vec<CompiledPolicy> = Vec::with_capacity(loaded.len());
        for (policy, default) in loaded {
            if let Some(d) = default {
                new_default = d;
            }
            new_list.push(policy);
        }
        *self.inner.write() = new_list;
        *self.default.write() = new_default;
        tracing::info!(
            count = self.inner.read().len(),
            ?new_default,
            "policy store reloaded"
        );
        Ok(())
    }

    fn load_one(&self, path: &Path) -> Result<(CompiledPolicy, Option<Action>)> {
        let raw =
            std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let file: PolicyFile =
            serde_yaml::from_str(&raw).with_context(|| format!("parse yaml {}", path.display()))?;
        super::validator::validate(&file)
            .with_context(|| format!("validate {}", path.display()))?;
        let default_action = file
            .default_action
            .as_deref()
            .and_then(Action::from_str_lossy);
        let compiled = compile_policy(&file);
        Ok((compiled, default_action))
    }

    /// Return a snapshot: (default_action, all policies sorted by
    /// name).
    pub fn snapshot(&self) -> (Action, Vec<CompiledPolicy>) {
        let guard = self.inner.read();
        let default = *self.default.read();
        (default, guard.clone())
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }
}
