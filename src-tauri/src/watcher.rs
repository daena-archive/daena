// Filesystem watcher and portable snapshots.
use super::*;

#[derive(Default)]
pub(super) struct ProjectWatcher {
    pub(super) stop: Option<mpsc::Sender<()>>,
    pub(super) filesystem: Option<RecommendedWatcher>,
}

pub(super) type SharedProjectWatcher = Arc<Mutex<ProjectWatcher>>;

pub(super) fn stop_project_watcher(watcher: &SharedProjectWatcher) -> Result<(), String> {
    let stop = {
        let mut watcher = watcher
            .lock()
            .map_err(|_| "project watcher lock poisoned".to_string())?;
        let stop = watcher.stop.take();
        watcher.filesystem.take();
        stop
    };
    if let Some(stop) = stop {
        let _ = stop.send(());
    }
    Ok(())
}

pub(super) fn start_project_watcher(
    app: &tauri::AppHandle,
    state: &SharedCore,
    watcher: &SharedProjectWatcher,
) -> Result<(), String> {
    stop_project_watcher(watcher)?;
    current_session(state)?
        .core
        .lock()
        .map_err(|_| "core lock poisoned".to_string())?
        .info()
        .ok_or_else(|| "project is not open".to_string())?;
    let (stop, receiver) = mpsc::channel();
    let (event_sender, event_receiver) = mpsc::channel();
    let root = current_info(state)?
        .ok_or_else(|| "project is not open".to_string())?
        .root;
    let startup_snapshot = portable_tree_snapshot(std::path::Path::new(&root))?;
    let watched_root = root.clone();
    let callback_root = watched_root.clone();
    let mut filesystem = RecommendedWatcher::new(
        move |event: notify::Result<notify::Event>| {
            let paths = event
                .ok()
                .into_iter()
                .flat_map(|event| event.paths)
                .filter_map(|path| {
                    watched_portable_path(std::path::Path::new(&callback_root), &path)
                })
                .collect::<BTreeSet<_>>();
            if !paths.is_empty() {
                let _ = event_sender.send(paths.into_iter().collect::<Vec<_>>());
            }
        },
        notify::Config::default(),
    )
    .map_err(|error| format!("start filesystem watcher: {error}"))?;
    filesystem
        .watch(std::path::Path::new(&root), RecursiveMode::Recursive)
        .map_err(|error| format!("watch project root: {error}"))?;
    watcher
        .lock()
        .map_err(|_| "project watcher lock poisoned".to_string())?
        .stop = Some(stop);
    watcher
        .lock()
        .map_err(|_| "project watcher lock poisoned".to_string())?
        .filesystem = Some(filesystem);
    let app = app.clone();
    let watched_core = state.clone();
    let startup_filter_until = Instant::now() + Duration::from_secs(1);
    thread::spawn(move || loop {
        match receiver.try_recv() {
            Ok(()) | Err(mpsc::TryRecvError::Disconnected) => return,
            Err(mpsc::TryRecvError::Empty) => {}
        }
        let mut paths = BTreeSet::new();
        match event_receiver.recv_timeout(Duration::from_millis(500)) {
            Ok(batch) => paths.extend(batch),
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }
        while let Ok(batch) = event_receiver.try_recv() {
            paths.extend(batch);
        }
        // A lifecycle transition can stop this thread while it is draining a
        // queued batch. Do not publish that stale batch after the next project
        // has already become current.
        match receiver.try_recv() {
            Ok(()) | Err(mpsc::TryRecvError::Disconnected) => return,
            Err(mpsc::TryRecvError::Empty) => {}
        }
        if Instant::now() < startup_filter_until {
            paths.retain(|path| {
                portable_path_fingerprint(std::path::Path::new(&watched_root), path)
                    .ok()
                    .flatten()
                    != startup_snapshot.get(path).cloned()
            });
        }
        // Mutations are committed to SQLite before the asynchronous exporter
        // rewrites the portable tree. Do not classify that in-flight export
        // as an external edit.
        let app_export_pending = current_info(&watched_core)
            .ok()
            .flatten()
            .is_some_and(|info| info.sync.state == "pending");
        if !paths.is_empty() && app_export_pending {
            continue;
        }
        if !paths.is_empty() && portable_checkpoint_is_current(std::path::Path::new(&watched_root))
        {
            continue;
        }
        if !paths.is_empty() {
            let _ = app.emit("project-portable-files-changed", paths);
        }
    });
    Ok(())
}

pub(super) fn portable_tree_snapshot(
    root: &std::path::Path,
) -> Result<BTreeMap<String, String>, String> {
    let mut snapshot = BTreeMap::new();
    for relative in ["project.json", "entities", "plugins", "assets"] {
        let path = root.join(relative);
        if path.exists() {
            collect_portable_snapshot(root, &path, &mut snapshot)?;
        }
    }
    Ok(snapshot)
}

pub(super) fn collect_portable_snapshot(
    root: &std::path::Path,
    path: &std::path::Path,
    snapshot: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    let relative = path
        .strip_prefix(root)
        .map_err(|error| format!("portable snapshot path: {error}"))?
        .to_string_lossy()
        .replace('\\', "/");
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("portable snapshot metadata: {error}"))?;
    if metadata.is_dir() {
        snapshot.insert(relative, "directory".into());
        let entries =
            fs::read_dir(path).map_err(|error| format!("portable snapshot directory: {error}"))?;
        for entry in entries {
            let entry = entry.map_err(|error| format!("portable snapshot entry: {error}"))?;
            collect_portable_snapshot(root, &entry.path(), snapshot)?;
        }
    } else if metadata.is_file() {
        let bytes = fs::read(path).map_err(|error| format!("portable snapshot file: {error}"))?;
        snapshot.insert(relative, format!("file:{:x}", Sha256::digest(bytes)));
    }
    Ok(())
}

pub(super) fn portable_path_fingerprint(
    root: &std::path::Path,
    relative: &str,
) -> Result<Option<String>, String> {
    let path = root.join(relative);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("portable path metadata: {error}")),
    };
    if metadata.is_dir() {
        return Ok(Some("directory".into()));
    }
    if metadata.is_file() {
        let bytes = fs::read(path).map_err(|error| format!("portable path file: {error}"))?;
        return Ok(Some(format!("file:{:x}", Sha256::digest(bytes))));
    }
    Ok(Some("other".into()))
}

pub(super) fn portable_checkpoint_is_current(root: &std::path::Path) -> bool {
    let checkpoint_path = root.join(daena_core::CHECKPOINT_MANIFEST_FILE);
    let Ok(checkpoint) = read_json::<CheckpointManifest>(&checkpoint_path) else {
        return false;
    };
    validate_checkpoint(root, &checkpoint).is_ok()
}

pub(super) fn watched_portable_path(
    root: &std::path::Path,
    path: &std::path::Path,
) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let mut components = relative.components();
    let first = components.next()?.as_os_str().to_str()?;
    if matches!(first, ".daena" | ".git")
        || relative
            .components()
            .any(|component| matches!(component.as_os_str().to_str(), Some(".daena" | ".git")))
    {
        return None;
    }
    let filename = relative.file_name()?.to_str()?;
    if filename == ".DS_Store"
        || filename.starts_with(".~")
        || filename.ends_with('~')
        || filename.ends_with(".swp")
        || filename.ends_with(".tmp")
    {
        return None;
    }
    if !matches!(first, "project.json" | "entities" | "plugins" | "assets") {
        return None;
    }
    Some(relative.to_string_lossy().replace('\\', "/"))
}
