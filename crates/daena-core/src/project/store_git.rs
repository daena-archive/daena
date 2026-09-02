// Built-in Git operations over portable files.
use super::*;

impl ProjectStore {
    pub fn git_commit(
        &self,
        message: String,
        paths: Option<Vec<String>>,
    ) -> Result<GitStatus, CoreError> {
        self.flush_checkpoint("git commit")?;
        self.git_commit_after_checkpoint(message, paths)
    }

    pub fn git_commit_after_checkpoint(
        &self,
        message: String,
        paths: Option<Vec<String>>,
    ) -> Result<GitStatus, CoreError> {
        if message.trim().is_empty() {
            return Err(CoreError::NotFound("commit message cannot be empty".into()));
        }
        let preflight = self.git_preflight_after_checkpoint()?;
        if !preflight.ready {
            return Err(CoreError::Conflict(format!(
                "cannot commit while canonical diagnostics remain: {}",
                preflight.diagnostics.join("; ")
            )));
        }
        if preflight.staging_paths.is_empty() {
            return Err(CoreError::Git(
                "no canonical project changes to commit".into(),
            ));
        }
        let selected = match paths {
            Some(paths) if paths.is_empty() => {
                return Err(CoreError::Git("no paths selected for commit".into()));
            }
            Some(paths) => {
                let allowed = preflight
                    .staging_paths
                    .iter()
                    .cloned()
                    .collect::<BTreeSet<_>>();
                for path in &paths {
                    if !allowed.contains(path) {
                        return Err(CoreError::Git(format!(
                            "path is not in the canonical staging preview: {path}"
                        )));
                    }
                    if !Self::is_canonical_git_path(path) {
                        return Err(CoreError::Git(format!(
                            "path is not a canonical project path: {path}"
                        )));
                    }
                }
                paths
            }
            None => preflight.staging_paths.clone(),
        };
        // Rebuild the canonical portion of the index so a selective commit
        // cannot accidentally include canonical paths staged before the UI
        // preview. `git reset` without a worktree target preserves all edits.
        let mut reset_args = vec![
            "reset".to_string(),
            "--mixed".to_string(),
            "HEAD".to_string(),
            "--".to_string(),
        ];
        reset_args.extend(preflight.staging_paths.iter().cloned());
        let reset_args = reset_args.iter().map(String::as_str).collect::<Vec<_>>();
        let reset = self.run_git(&reset_args)?;
        if !reset.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&reset.stderr).trim().into(),
            ));
        }

        let mut add_args = vec!["add".to_string(), "--all".to_string(), "--".to_string()];
        add_args.extend(selected);
        let add_args = add_args.iter().map(String::as_str).collect::<Vec<_>>();
        let add = self.run_git(&add_args)?;
        if !add.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&add.stderr).trim().into(),
            ));
        }
        let commit = self.run_git(&["commit", "-m", message.trim()])?;
        if !commit.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&commit.stderr).trim().into(),
            ));
        }
        self.git_status()
    }

    pub fn git_init(&self) -> Result<GitStatus, CoreError> {
        let output = self.run_git(&["init"])?;
        if !output.status.success() {
            return Err(CoreError::NotFound(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        self.invalidate_git_repository_probe();
        self.git_status()
    }

    pub fn git_log(&self) -> Result<Vec<GitLogEntry>, CoreError> {
        if !self.git_status()?.repository {
            return Ok(Vec::new());
        }
        let output = self.run_git(&[
            "log",
            "-50",
            "--date=iso-strict",
            "--pretty=format:%h%x09%ad%x09%s",
        ])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("does not have any commits yet") {
                return Ok(Vec::new());
            }
            return Err(CoreError::NotFound(stderr.trim().into()));
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                let mut parts = line.splitn(3, '\t');
                Some(GitLogEntry {
                    hash: parts.next()?.into(),
                    date: parts.next()?.into(),
                    subject: parts.next()?.into(),
                })
            })
            .collect())
    }

    pub fn git_preflight(&self) -> Result<GitPreflight, CoreError> {
        self.flush_checkpoint("git preflight")?;
        self.git_preflight_after_checkpoint()
    }

    pub fn git_preflight_after_checkpoint(&self) -> Result<GitPreflight, CoreError> {
        let status = self.git_status()?;
        if !status.repository {
            return Ok(GitPreflight {
                ready: false,
                diagnostics: vec!["project is not a git repository".into()],
                canonical_paths: Vec::new(),
                asset_paths: Vec::new(),
                staging_paths: Vec::new(),
                staged_paths: Vec::new(),
                unmerged_paths: Vec::new(),
            });
        }

        let entries = self.git_status_entries()?;
        let mut diagnostics = Vec::new();
        let unmerged_paths = entries
            .iter()
            .filter(|(index_status, worktree_status, _)| {
                matches!(
                    (*index_status, *worktree_status),
                    (b'D' | b'U', b'D') | (b'A' | b'D' | b'U', b'U') | (b'U' | b'A', b'A')
                )
            })
            .map(|(_, _, path)| path.clone())
            .collect::<Vec<_>>();
        if !unmerged_paths.is_empty() {
            diagnostics.extend(
                unmerged_paths
                    .iter()
                    .map(|path| format!("git.unmerged: {path}")),
            );
        }

        let report = if unmerged_paths.is_empty() {
            ExternalChangeReport {
                changed: false,
                paths: Vec::new(),
                diagnostics: Vec::new(),
            }
        } else {
            ExternalChangeReport {
                changed: false,
                paths: unmerged_paths.clone(),
                diagnostics: Vec::new(),
            }
        };
        diagnostics.extend(report.diagnostics);
        let sync = self.sync_summary()?;
        if sync.state != "clean" {
            diagnostics.push(format!("index.state: {}", sync.state));
        }

        // The flush may have created or replaced portable paths. Re-read Git
        // state before constructing the staging preview so queued exports are
        // visible to the caller.
        let entries = self.git_status_entries()?;
        let staged_paths = self.git_staged_paths()?;

        let noncanonical_staged = staged_paths
            .iter()
            .filter(|path| !Self::is_canonical_git_path(path))
            .cloned()
            .collect::<Vec<_>>();
        if !noncanonical_staged.is_empty() {
            diagnostics.push(format!(
                "git.noncanonical-staged: {}",
                noncanonical_staged.join(", ")
            ));
        }

        let canonical_paths = entries
            .iter()
            .filter(|(_, _, path)| Self::is_canonical_git_path(path))
            .map(|(_, _, path)| path.clone())
            .chain(
                staged_paths
                    .iter()
                    .filter(|path| Self::is_canonical_git_path(path))
                    .cloned(),
            )
            .collect::<BTreeSet<_>>();
        let mut canonical_paths = canonical_paths.iter().cloned().collect::<Vec<_>>();
        let asset_paths = canonical_paths
            .iter()
            .filter(|path| Self::is_asset_git_path(path))
            .cloned()
            .collect::<Vec<_>>();
        let staging_paths = canonical_paths.clone();
        canonical_paths.shrink_to_fit();

        Ok(GitPreflight {
            ready: diagnostics.is_empty(),
            diagnostics,
            canonical_paths,
            asset_paths,
            staging_paths,
            staged_paths,
            unmerged_paths,
        })
    }

    pub fn git_push(
        &self,
        remote: &str,
        branch: Option<&str>,
        force_with_lease: bool,
    ) -> Result<GitStatus, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        Self::validate_remote_name(remote)?;
        let branch = match branch {
            Some(branch) if !branch.trim().is_empty() => branch.trim().to_string(),
            _ => {
                let status = self.git_status()?;
                status
                    .branch
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| CoreError::Git("current branch is required for push".into()))?
            }
        };
        let mut args = vec!["push".to_string()];
        if force_with_lease {
            args.push("--force-with-lease".into());
        } else if self.git_upstream()?.is_none() {
            // The first ordinary push should establish the upstream that the
            // restore/recovery workflow relies on. Never rewrite an existing
            // upstream when pushing to an additional remote.
            args.push("--set-upstream".into());
        }
        args.push(remote.trim().into());
        args.push(branch);
        let args = args.iter().map(String::as_str).collect::<Vec<_>>();
        let output = self.run_git(&args)?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        self.git_status()
    }

    pub fn git_remote_add(&self, name: &str, url: &str) -> Result<Vec<GitRemote>, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        Self::validate_remote_name(name)?;
        Self::validate_remote_url(url)?;
        let output = self.run_git(&["remote", "add", name.trim(), url.trim()])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        self.git_remote_list()
    }

    pub fn git_remote_list(&self) -> Result<Vec<GitRemote>, CoreError> {
        if !self.git_status()?.repository {
            return Ok(Vec::new());
        }
        let output = self.run_git(&["remote", "-v"])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        let mut remotes = BTreeMap::<String, GitRemote>::new();
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let mut parts = line.split_whitespace();
            let Some(name) = parts.next() else { continue };
            let Some(url) = parts.next() else { continue };
            let mode = parts.next().unwrap_or("(fetch)");
            let entry = remotes.entry(name.to_string()).or_insert(GitRemote {
                name: name.to_string(),
                fetch_url: String::new(),
                push_url: String::new(),
            });
            if mode.contains("push") {
                entry.push_url = url.to_string();
            } else {
                entry.fetch_url = url.to_string();
            }
        }
        for remote in remotes.values_mut() {
            if remote.push_url.is_empty() {
                remote.push_url = remote.fetch_url.clone();
            }
            if remote.fetch_url.is_empty() {
                remote.fetch_url = remote.push_url.clone();
            }
        }
        Ok(remotes.into_values().collect())
    }

    pub fn git_remote_remove(&self, name: &str) -> Result<Vec<GitRemote>, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        Self::validate_remote_name(name)?;
        let output = self.run_git(&["remote", "remove", name.trim()])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        self.git_remote_list()
    }

    pub fn git_remote_set_url(&self, name: &str, url: &str) -> Result<Vec<GitRemote>, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        Self::validate_remote_name(name)?;
        Self::validate_remote_url(url)?;
        let output = self.run_git(&["remote", "set-url", name.trim(), url.trim()])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        self.git_remote_list()
    }

    pub(crate) fn git_repository_is_project_root_cached(
        &self,
        force: bool,
    ) -> Result<bool, CoreError> {
        let root = self.project_root()?.to_path_buf();
        if !force {
            if let Ok(cache) = git_repository_probe_slot().lock() {
                if cache.get(&root) == Some(&false) {
                    return Ok(false);
                }
            }
        }
        let output = self.run_git(&["rev-parse", "--show-toplevel"])?;
        let repository = if !output.status.success() {
            false
        } else {
            let repository_root = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
            let project_root = self.project_root()?;
            let repository_root =
                std::fs::canonicalize(&repository_root).unwrap_or(repository_root);
            let project_root =
                std::fs::canonicalize(project_root).unwrap_or_else(|_| project_root.into());
            repository_root == project_root
        };
        if let Ok(mut cache) = git_repository_probe_slot().lock() {
            cache.insert(root, repository);
        }
        Ok(repository)
    }

    pub fn git_reset_hard(&mut self, hash: &str) -> Result<GitResetResult, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        let preflight = self.git_preflight()?;
        if !preflight.ready {
            return Err(CoreError::Conflict(format!(
                "cannot reset while canonical diagnostics remain: {}",
                preflight.diagnostics.join("; ")
            )));
        }
        let hash = hash.trim();
        if hash.is_empty() {
            return Err(CoreError::Validation("commit hash cannot be empty".into()));
        }
        let previous_head = self.git_rev_parse("HEAD")?;
        let verify = self.run_git(&["rev-parse", "--verify", hash])?;
        if !verify.status.success() {
            return Err(CoreError::Git(format!(
                "unknown commit: {}",
                String::from_utf8_lossy(&verify.stderr).trim()
            )));
        }
        let upstream_before = self.git_upstream()?;
        let reset = self.run_git(&["reset", "--hard", hash])?;
        if !reset.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&reset.stderr).trim().into(),
            ));
        }
        let root = self.project_root()?.to_path_buf();
        let generation: Generation = self.connection.query_row(
            "SELECT content_generation FROM runtime_meta WHERE key='runtime'",
            [],
            |row| row.get(0),
        )?;
        self.install_checkpoint_manifest(&root, generation)?;
        let rebuild = self.import_checkpoint()?;
        let current_head = self.git_rev_parse("HEAD")?;
        let upstream = self.git_upstream()?.or(upstream_before);
        let diverged_from_upstream = match (&upstream, &current_head) {
            (Some(upstream), Some(head)) => upstream
                .remote_hash
                .as_ref()
                .is_some_and(|remote| remote != head),
            (Some(_), _) => true,
            _ => false,
        };
        Ok(GitResetResult {
            status: self.git_status()?,
            previous_head,
            current_head,
            upstream,
            diverged_from_upstream,
            rebuild,
        })
    }

    pub fn git_restore_from_upstream(&mut self) -> Result<GitResetResult, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        let preflight = self.git_preflight()?;
        if !preflight.ready {
            return Err(CoreError::Conflict(format!(
                "cannot restore upstream while canonical diagnostics remain: {}",
                preflight.diagnostics.join("; ")
            )));
        }
        let upstream = self
            .git_upstream()?
            .ok_or_else(|| CoreError::Git("no upstream branch is configured for restore".into()))?;
        let previous_head = self.git_rev_parse("HEAD")?;
        let fetch = self.run_git(&["fetch", &upstream.remote, &upstream.branch])?;
        if !fetch.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&fetch.stderr).trim().into(),
            ));
        }
        let upstream_ref = format!("{}/{}", upstream.remote, upstream.branch);
        let reset = self.run_git(&["reset", "--hard", &upstream_ref])?;
        if !reset.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&reset.stderr).trim().into(),
            ));
        }
        let root = self.project_root()?.to_path_buf();
        let generation: Generation = self.connection.query_row(
            "SELECT content_generation FROM runtime_meta WHERE key='runtime'",
            [],
            |row| row.get(0),
        )?;
        self.install_checkpoint_manifest(&root, generation)?;
        let rebuild = self.import_checkpoint()?;
        let current_head = self.git_rev_parse("HEAD")?;
        let upstream = self.git_upstream()?;
        Ok(GitResetResult {
            status: self.git_status()?,
            previous_head,
            current_head,
            upstream,
            diverged_from_upstream: false,
            rebuild,
        })
    }

    pub(crate) fn git_rev_parse(&self, rev: &str) -> Result<Option<String>, CoreError> {
        let output = self.run_git(&["rev-parse", rev])?;
        if !output.status.success() {
            return Ok(None);
        }
        let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok((!hash.is_empty()).then_some(hash))
    }

    pub fn git_show_changes(&self, hash: &str) -> Result<Vec<GitChange>, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        let hash = hash.trim();
        if hash.is_empty() {
            return Err(CoreError::Validation("commit hash cannot be empty".into()));
        }
        let output = self.run_git(&[
            "diff-tree",
            "--root",
            "--no-commit-id",
            "--name-status",
            "-r",
            "--find-renames",
            hash,
        ])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                let mut parts = line.split('\t');
                let status = parts.next()?.trim();
                let first_path = parts.next()?.trim();
                let path = if status.starts_with('R') || status.starts_with('C') {
                    parts.next().unwrap_or(first_path).trim()
                } else {
                    first_path
                };
                if status.is_empty() || path.is_empty() || !Self::is_canonical_git_path(path) {
                    return None;
                }
                Some(GitChange {
                    status: status.into(),
                    path: path.into(),
                })
            })
            .collect())
    }

    pub fn git_show_diff(&self, hash: &str, path: &str) -> Result<String, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        let hash = hash.trim();
        let path = path.trim();
        if hash.is_empty() || path.is_empty() {
            return Err(CoreError::Validation(
                "commit hash and path are required".into(),
            ));
        }
        if !Self::is_canonical_git_path(path) {
            return Err(CoreError::Validation(
                "snapshot diffs are limited to canonical paths".into(),
            ));
        }
        let output = self.run_git(&[
            "diff-tree",
            "--root",
            "-p",
            "--no-commit-id",
            hash,
            "--",
            path,
        ])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into())
    }

    pub fn git_show_file(&self, hash: &str, path: &str) -> Result<String, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        let hash = hash.trim();
        let path = path.trim();
        if hash.is_empty() || path.is_empty() {
            return Err(CoreError::Validation(
                "commit hash and path are required".into(),
            ));
        }
        if !Self::is_canonical_git_path(path) {
            return Err(CoreError::Validation(format!(
                "path is not a canonical project path: {path}"
            )));
        }
        if path.contains('\0') || path.starts_with('-') {
            return Err(CoreError::Validation("invalid snapshot path".into()));
        }
        let spec = format!("{hash}:{path}");
        let output = self.run_git(&["show", &spec])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        if output.stdout.len() > GIT_SHOW_FILE_MAX_BYTES {
            return Err(CoreError::Validation(format!(
                "snapshot file is too large to preview ({} bytes)",
                output.stdout.len()
            )));
        }
        if output.stdout.contains(&0) {
            return Err(CoreError::Validation(
                "binary snapshot files cannot be previewed".into(),
            ));
        }
        String::from_utf8(output.stdout)
            .map_err(|_| CoreError::Validation("snapshot file is not valid UTF-8 text".into()))
    }

    pub fn git_show_message(&self, hash: &str) -> Result<String, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        let hash = hash.trim();
        if hash.is_empty() {
            return Err(CoreError::Validation("commit hash cannot be empty".into()));
        }
        let output = self.run_git(&["show", "-s", "--format=%B", hash])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().into())
    }

    pub fn git_show_tree(&self, hash: &str) -> Result<Vec<String>, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        let hash = hash.trim();
        if hash.is_empty() {
            return Err(CoreError::Validation("commit hash cannot be empty".into()));
        }
        let output = self.run_git(&["ls-tree", "-r", "--name-only", hash])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .filter(|path| !path.is_empty() && Self::is_canonical_git_path(path))
            .map(str::to_string)
            .collect())
    }

    pub(crate) fn git_staged_paths(&self) -> Result<Vec<String>, CoreError> {
        let output = self.run_git(&["diff", "--cached", "--name-only", "-z"])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }
        Ok(output
            .stdout
            .split(|byte| *byte == 0)
            .filter(|path| !path.is_empty())
            .map(|path| String::from_utf8_lossy(path).into_owned())
            .collect())
    }

    pub fn git_staging_preview(&self) -> Result<GitPreflight, CoreError> {
        self.git_preflight()
    }

    /// Report snapshot repository status for the project.
    ///
    /// The "project is not a git repository" probe is cached per project root
    /// for the lifetime of the process, so repeated status refreshes do not
    /// spawn git. The cache is refreshed by [`Self::git_status_reprobe`] (used
    /// when the snapshots UI is opened) and invalidated by [`Self::git_init`].
    /// A repository created outside the app is therefore not reported here
    /// until the snapshots UI re-probes.
    pub fn git_status(&self) -> Result<GitStatus, CoreError> {
        self.git_status_with_probe(false)
    }

    pub(crate) fn git_status_entries(&self) -> Result<Vec<(u8, u8, String)>, CoreError> {
        let output = self.run_git(&["status", "--porcelain=v1", "-z", "--untracked-files=all"])?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }
        let mut entries = Vec::new();
        let mut records = output.stdout.split(|byte| *byte == 0);
        while let Some(record) = records.next() {
            if record.len() < 4 || record[2] != b' ' {
                continue;
            }
            let index_status = record[0];
            let worktree_status = record[1];
            // With porcelain v1's `-z` format, a rename/copy record stores the
            // destination first and the source in the following NUL-delimited
            // field. Keep the destination as the actionable canonical path.
            let path = String::from_utf8_lossy(&record[3..]).into_owned();
            if matches!(index_status, b'R' | b'C') || matches!(worktree_status, b'R' | b'C') {
                let _source_path = records.next();
            }
            if !path.is_empty() {
                entries.push((index_status, worktree_status, path));
            }
        }
        Ok(entries)
    }

    /// Report snapshot repository status, bypassing a cached negative probe.
    pub fn git_status_reprobe(&self) -> Result<GitStatus, CoreError> {
        self.git_status_with_probe(true)
    }

    pub(crate) fn git_status_with_probe(&self, force_probe: bool) -> Result<GitStatus, CoreError> {
        // Git walks up parent directories by default. Built-in snapshots must
        // never attach to or mutate a repository that owns a broader worktree.
        if !self.git_repository_is_project_root_cached(force_probe)? {
            return Ok(GitStatus {
                repository: false,
                branch: None,
                changes: Vec::new(),
                canonical_changes: Vec::new(),
                staged_canonical_changes: Vec::new(),
            });
        }
        let branch = self.run_git(&["branch", "--show-current"])?;
        let entries = self.git_status_entries()?;
        let changes = entries
            .iter()
            .map(|(index_status, worktree_status, path)| {
                format!(
                    "{}{} {}",
                    *index_status as char, *worktree_status as char, path
                )
            })
            .collect::<Vec<_>>();
        let canonical_changes = entries
            .iter()
            .filter(|(_, _, path)| Self::is_canonical_git_path(path))
            .map(|(_, _, path)| path.clone())
            .collect::<Vec<_>>();
        let staged_canonical_changes = self
            .git_staged_paths()?
            .into_iter()
            .filter(|path| Self::is_canonical_git_path(path))
            .collect();
        Ok(GitStatus {
            repository: true,
            branch: Some(String::from_utf8_lossy(&branch.stdout).trim().to_string()),
            changes,
            canonical_changes,
            staged_canonical_changes,
        })
    }

    pub fn git_super_squash_after_checkpoint(
        &self,
        message: &str,
    ) -> Result<GitResetResult, CoreError> {
        if message.trim().is_empty() {
            return Err(CoreError::NotFound(
                "snapshot message cannot be empty".into(),
            ));
        }
        let preflight = self.git_preflight_after_checkpoint()?;
        if !preflight.ready {
            return Err(CoreError::Conflict(format!(
                "cannot squash while canonical diagnostics remain: {}",
                preflight.diagnostics.join("; ")
            )));
        }
        if !preflight.staging_paths.is_empty() {
            return Err(CoreError::Git(
                "commit snapshot-ready changes before squashing history".into(),
            ));
        }
        let head = self.run_git(&["rev-parse", "--verify", "HEAD"])?;
        if !head.status.success() {
            return Err(CoreError::Git("no snapshot history to squash".into()));
        }
        let previous_head = self.git_rev_parse("HEAD")?;
        let read_tree = self.run_git(&["read-tree", "HEAD"])?;
        if !read_tree.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&read_tree.stderr).trim().into(),
            ));
        }
        let tree = self.run_git(&["write-tree"])?;
        if !tree.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&tree.stderr).trim().into(),
            ));
        }
        let tree_hash = String::from_utf8_lossy(&tree.stdout).trim().to_owned();
        let commit = self.run_git(&["commit-tree", &tree_hash, "-m", message.trim()])?;
        if !commit.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&commit.stderr).trim().into(),
            ));
        }
        let commit_hash = String::from_utf8_lossy(&commit.stdout).trim().to_owned();
        let reset = self.run_git(&["reset", "--soft", &commit_hash])?;
        if !reset.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&reset.stderr).trim().into(),
            ));
        }
        let current_head = self.git_rev_parse("HEAD")?;
        let upstream = self.git_upstream()?;
        let diverged_from_upstream = match (&upstream, &current_head) {
            (Some(upstream), Some(head)) => upstream
                .remote_hash
                .as_ref()
                .is_some_and(|remote| remote != head),
            (Some(_), _) => true,
            _ => false,
        };
        Ok(GitResetResult {
            status: self.git_status()?,
            previous_head,
            current_head,
            upstream,
            diverged_from_upstream,
            rebuild: ExternalChangeReport {
                changed: false,
                paths: Vec::new(),
                diagnostics: Vec::new(),
            },
        })
    }

    /// Probe the system Git version (`git --version`).
    ///
    /// A successful probe is cached for the lifetime of the process: the
    /// version cannot change while Daena runs, and the probe spawns a
    /// subprocess, which is slow on Windows. Unavailable probes are never
    /// cached so an install-then-retry flow re-probes.
    #[must_use]
    pub fn git_tool_info() -> GitToolInfo {
        if let Ok(cache) = git_tool_info_slot().lock() {
            if let Some(cached) = cache.as_ref() {
                return cached.clone();
            }
        }
        let info = match git_command(&["--version"]).output() {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                GitToolInfo {
                    available: true,
                    version: Some(version),
                    error: None,
                }
            }
            Ok(output) => {
                let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
                GitToolInfo {
                    available: false,
                    version: None,
                    error: Some(if error.is_empty() {
                        "git --version failed".into()
                    } else {
                        error
                    }),
                }
            }
            Err(error) => GitToolInfo {
                available: false,
                version: None,
                error: Some(format!("git is unavailable: {error}")),
            },
        };
        if info.available {
            if let Ok(mut cache) = git_tool_info_slot().lock() {
                *cache = Some(info.clone());
            }
        }
        info
    }

    pub(crate) fn git_upstream(&self) -> Result<Option<GitUpstream>, CoreError> {
        let remote =
            self.run_git(&["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])?;
        if !remote.status.success() {
            return Ok(None);
        }
        let full = String::from_utf8_lossy(&remote.stdout).trim().to_string();
        let Some((remote_name, branch)) = full.split_once('/') else {
            return Ok(None);
        };
        let remote_hash = self.git_rev_parse("@{u}")?;
        Ok(Some(GitUpstream {
            remote: remote_name.to_string(),
            branch: branch.to_string(),
            remote_hash,
        }))
    }

    pub fn git_worktree_diff(&self, paths: &[String]) -> Result<String, CoreError> {
        if !self.git_status()?.repository {
            return Err(CoreError::Git("project is not a git repository".into()));
        }
        if paths.is_empty() {
            return Err(CoreError::Validation(
                "at least one diff path is required".into(),
            ));
        }
        if paths.iter().any(|path| !Self::is_canonical_git_path(path)) {
            return Err(CoreError::Validation(
                "snapshot diffs are limited to canonical paths".into(),
            ));
        }
        let head = self.run_git(&["rev-parse", "--verify", "HEAD"])?;
        if !head.status.success() {
            let mut combined = String::new();
            for path in paths {
                let output = self.run_git(&[
                    "diff",
                    "--no-index",
                    "--no-color",
                    "--no-ext-diff",
                    "--unified=3",
                    "/dev/null",
                    path,
                ])?;
                if !output.status.success() && output.status.code() != Some(1) {
                    return Err(CoreError::Git(
                        String::from_utf8_lossy(&output.stderr).trim().into(),
                    ));
                }
                combined.push_str(&String::from_utf8_lossy(&output.stdout));
            }
            return Ok(combined);
        }
        let mut args = vec![
            "diff".to_string(),
            "HEAD".to_string(),
            "--no-color".to_string(),
            "--no-ext-diff".to_string(),
            "--unified=3".to_string(),
            "--".to_string(),
        ];
        args.extend(paths.iter().cloned());
        let args = args.iter().map(String::as_str).collect::<Vec<_>>();
        let output = self.run_git(&args)?;
        if !output.status.success() {
            return Err(CoreError::Git(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into())
    }

    pub(crate) fn run_git(&self, args: &[&str]) -> Result<std::process::Output, CoreError> {
        let root = self.project_root()?;
        git_command(args)
            .current_dir(root)
            .output()
            .map_err(|error| CoreError::NotFound(format!("git is unavailable: {error}")))
    }

    pub(crate) fn invalidate_git_repository_probe(&self) {
        let Some(root) = self.root.as_deref() else {
            return;
        };
        if let Ok(mut cache) = git_repository_probe_slot().lock() {
            cache.remove(root);
        }
    }

    pub(crate) fn is_canonical_git_path(path: &str) -> bool {
        path == "project.json"
            || path == ".gitignore"
            || path.starts_with("entities/")
            || path.starts_with("plugins/")
            || path.starts_with("assets/")
    }

    pub(crate) fn is_asset_git_path(path: &str) -> bool {
        path.starts_with("assets/")
    }

    pub(crate) fn validate_remote_url(url: &str) -> Result<(), CoreError> {
        let url = url.trim();
        if url.is_empty() {
            return Err(CoreError::Validation("remote URL cannot be empty".into()));
        }
        let ok = url.starts_with("https://")
            || url.starts_with("http://")
            || url.starts_with("ssh://")
            || url.starts_with("git://")
            || url.starts_with("git@")
            || url.starts_with("file://")
            || Path::new(url).is_absolute();
        if !ok {
            return Err(CoreError::Validation(format!(
                "unsupported remote URL: {url}"
            )));
        }
        Ok(())
    }

    pub(crate) fn validate_remote_name(name: &str) -> Result<(), CoreError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(CoreError::Validation("remote name cannot be empty".into()));
        }
        if !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        {
            return Err(CoreError::Validation(format!(
                "invalid remote name: {name}"
            )));
        }
        Ok(())
    }
}
