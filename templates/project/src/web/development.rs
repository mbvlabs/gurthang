use std::{
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
    time::Duration,
};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tower_livereload::Reloader;

use crate::{
    error::{AppError, Result},
    web::tera::TeraEngine,
};

pub struct DevelopmentWatcher {
    watcher: Option<RecommendedWatcher>,
    worker: Option<thread::JoinHandle<()>>,
}

impl DevelopmentWatcher {
    pub fn start(templates: TeraEngine, reloader: Reloader) -> Result<Self> {
        let (sender, receiver) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
            let _ = sender.send(event);
        })
        .map_err(development_error)?;

        for path in [Path::new("templates"), Path::new("assets/css")] {
            watcher
                .watch(path, RecursiveMode::Recursive)
                .map_err(development_error)?;
        }

        let worker = thread::spawn(move || {
            while let Ok(event) = receiver.recv() {
                let mut events = vec![event];
                while let Ok(event) = receiver.recv_timeout(Duration::from_millis(75)) {
                    events.push(event);
                }

                let paths: Vec<_> = changed_paths(events)
                    .into_iter()
                    .filter(|path| reload_path(path))
                    .collect();
                if paths.is_empty() {
                    continue;
                }

                let templates_changed = paths.iter().any(|path| {
                    path.extension()
                        .is_some_and(|extension| extension == "html")
                });
                if templates_changed && let Err(error) = templates.reload() {
                    tracing::warn!(%error, "template reload failed; keeping the last valid templates");
                    continue;
                }

                tracing::info!(?paths, "development files changed; reloading browser");
                reloader.reload();
            }
        });

        Ok(Self {
            watcher: Some(watcher),
            worker: Some(worker),
        })
    }
}

impl Drop for DevelopmentWatcher {
    fn drop(&mut self) {
        self.watcher.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn changed_paths(events: Vec<notify::Result<Event>>) -> Vec<PathBuf> {
    events
        .into_iter()
        .filter_map(|event| match event {
            Ok(event) if reload_worthy(&event.kind) => Some(event.paths),
            Ok(_) => None,
            Err(error) => {
                tracing::warn!(%error, "development file watcher error");
                None
            }
        })
        .flatten()
        .collect()
}

fn reload_worthy(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Any | EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

fn reload_path(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == "html")
        || path
            .file_name()
            .is_some_and(|file_name| file_name == "style.css")
}

fn development_error(error: notify::Error) -> AppError {
    AppError::Development(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reload_paths_cover_templates_and_compiled_styles_only() {
        assert!(reload_path(Path::new("/app/templates/pages/home.html")));
        assert!(reload_path(Path::new("/app/assets/css/style.css")));
        assert!(!reload_path(Path::new("/app/templates/.home.html.swp")));
        assert!(!reload_path(Path::new("/app/assets/css/unrelated.css")));
    }
}
