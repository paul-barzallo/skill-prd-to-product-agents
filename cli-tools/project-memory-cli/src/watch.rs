use crate::cli::{IngestArgs, WatchArgs};
use crate::model::{Snapshot, WatchIteration, WatchReport};
use crate::{scan, store, util};
use anyhow::{bail, Context, Result};
use notify::{Config, Event, PollWatcher, RecursiveMode, Watcher};
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

pub fn run(project_root: &Path, args: &WatchArgs) -> Result<(Vec<String>, WatchReport)> {
    if args.max_events == 0 {
        bail!("--max-events must be greater than zero");
    }
    if args.interval_ms == 0 {
        bail!("--interval-ms must be greater than zero");
    }

    let snapshot_exists = store::snapshot_path(project_root).is_file();
    let initial_snapshot_created = if !snapshot_exists || args.force_initial_ingest {
        scan::ingest(
            project_root,
            &IngestArgs {
                force: args.force_initial_ingest,
            },
        )?;
        true
    } else {
        false
    };

    let (tx, rx) = mpsc::channel();
    let mut watcher = PollWatcher::new(
        tx,
        Config::default().with_poll_interval(Duration::from_millis(args.interval_ms)),
    )
    .context("creating poll watcher")?;
    watcher
        .watch(project_root, RecursiveMode::Recursive)
        .with_context(|| format!("watching {}", project_root.display()))?;

    // PollWatcher needs one interval to establish the initial filesystem baseline.
    thread::sleep(Duration::from_millis(args.interval_ms));

    let started_at = Instant::now();
    let mut iterations = Vec::new();
    let mut timed_out = false;

    while iterations.len() < args.max_events {
        let wait_for = remaining_timeout(args.timeout_ms, started_at);
        let event = match wait_for {
            Some(duration) => match rx.recv_timeout(duration) {
                Ok(event) => event,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    timed_out = true;
                    break;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    bail!("watch channel disconnected unexpectedly");
                }
            },
            None => rx.recv().context("waiting for watch events")?,
        };

        let event = event.context("filesystem watch failed")?;
        let observed_paths = relevant_observed_paths(&event, project_root);
        if observed_paths.is_empty() {
            continue;
        }

        let previous = store::load_snapshot(project_root).ok();
        let (_warnings, ingest_report) = scan::ingest(project_root, &IngestArgs { force: false })?;
        let current = store::load_snapshot(project_root)?;
        let (changed_paths, deleted_paths) = diff_snapshots(previous.as_ref(), &current);

        iterations.push(WatchIteration {
            sequence: iterations.len() + 1,
            observed_paths,
            changed_paths,
            deleted_paths,
            ingest: ingest_report,
        });
    }

    let mut warnings = Vec::new();
    if timed_out {
        warnings.push("watch timed out before reaching max_events".to_string());
    }

    Ok((
        warnings,
        WatchReport {
            initial_snapshot_created,
            max_events: args.max_events,
            interval_ms: args.interval_ms,
            timeout_ms: args.timeout_ms,
            timed_out,
            events_observed: iterations.len(),
            iterations,
        },
    ))
}

fn remaining_timeout(timeout_ms: Option<u64>, started_at: Instant) -> Option<Duration> {
    timeout_ms.map(|timeout| {
        let total = Duration::from_millis(timeout);
        total.saturating_sub(started_at.elapsed())
    })
}

fn relevant_observed_paths(event: &Event, project_root: &Path) -> Vec<String> {
    let mut paths = BTreeSet::new();

    for path in &event.paths {
        let candidate = if path.is_absolute() {
            util::normalize_path(path)
        } else {
            util::normalize_path(&project_root.join(path))
        };

        if !candidate.starts_with(project_root) {
            continue;
        }

        let relative = util::to_relative_posix(&candidate, project_root);
        if should_skip_relative(&relative) {
            continue;
        }
        paths.insert(relative);
    }

    paths.into_iter().collect()
}

fn diff_snapshots(previous: Option<&Snapshot>, current: &Snapshot) -> (Vec<String>, Vec<String>) {
    let mut changed = BTreeSet::new();
    let mut deleted = BTreeSet::new();

    let current_by_path = current
        .files
        .iter()
        .map(|file| (file.path.as_str(), file.hash.as_str()))
        .collect::<std::collections::BTreeMap<_, _>>();

    match previous {
        Some(previous) => {
            let previous_by_path = previous
                .files
                .iter()
                .map(|file| (file.path.as_str(), file.hash.as_str()))
                .collect::<std::collections::BTreeMap<_, _>>();

            for (path, hash) in &current_by_path {
                match previous_by_path.get(path) {
                    Some(previous_hash) if previous_hash == hash => {}
                    _ => {
                        changed.insert((*path).to_string());
                    }
                }
            }

            for path in previous_by_path.keys() {
                if !current_by_path.contains_key(path) {
                    deleted.insert((*path).to_string());
                }
            }
        }
        None => {
            for path in current_by_path.keys() {
                changed.insert((*path).to_string());
            }
        }
    }

    (
        changed.into_iter().collect(),
        deleted.into_iter().collect(),
    )
}

fn should_skip_relative(relative: &str) -> bool {
    relative.starts_with(".git/")
        || relative.starts_with(".project-memory/")
        || relative.contains("/target/")
        || relative.contains("/target-staging/")
}
