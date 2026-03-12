// Git repository information for statusline display.
// Uses the `gix` crate for pure-Rust git detection without
// shelling out to the `git` binary.

pub struct GitInfo {
    pub branch: String,
    pub is_dirty: bool,
}

/// Detect git repository information for the given working directory.
///
/// Returns `None` if the directory is not inside a git repository
/// or if any git operation fails. This function never panics.
pub fn get_git_info(working_dir: &str) -> Option<GitInfo> {
    let repo = gix::discover(working_dir).ok()?;

    let branch = detect_branch(&repo)?;
    let is_dirty = detect_dirty(&repo).unwrap_or(false);

    Some(GitInfo { branch, is_dirty })
}

/// Read HEAD to determine the current branch name or short commit hash.
fn detect_branch(repo: &gix::Repository) -> Option<String> {
    let head = repo.head().ok()?;

    if head.is_detached() {
        // Detached HEAD: use the first 7 characters of the commit hash.
        let id = head.id()?;
        Some(id.to_string()[..7].to_string())
    } else {
        // Named branch: extract the short name from the full ref.
        let referent = head.referent_name()?;
        Some(referent.shorten().to_string())
    }
}

/// Check whether the working tree has uncommitted changes.
///
/// Compares the on-disk index against the HEAD tree. If the index
/// entry count differs from the HEAD tree entry count, or if any
/// index entry differs from its HEAD tree counterpart, the repo
/// is considered dirty.
///
/// Falls back to `false` on any error to avoid blocking the statusline.
fn detect_dirty(repo: &gix::Repository) -> Option<bool> {
    let index = repo.index_or_empty().ok()?;
    let head_tree = repo.head_tree_id().ok();

    // If there is no HEAD commit yet (empty repo) but the index has entries,
    // that means there are staged files — the repo is dirty.
    let head_tree = match head_tree {
        Some(id) => id,
        None => return Some(!index.entries().is_empty()),
    };

    let tree = head_tree.object().ok()?.into_tree();
    let mut head_entries: std::collections::HashMap<String, gix::ObjectId> =
        std::collections::HashMap::new();

    collect_tree_entries(repo, &tree, &mut String::new(), &mut head_entries);

    // Quick length check: different counts means something changed.
    if index.entries().len() != head_entries.len() {
        return Some(true);
    }

    // Compare each index entry against the HEAD tree.
    for entry in index.entries() {
        let path = entry.path(&index);
        let key = path.to_string();
        match head_entries.get(&key) {
            Some(tree_oid) => {
                if entry.id != *tree_oid {
                    return Some(true);
                }
            }
            None => return Some(true),
        }
    }

    Some(false)
}

/// Recursively collect all blob entries from a tree into a flat map
/// of path -> object id.
fn collect_tree_entries(
    _repo: &gix::Repository,
    tree: &gix::Tree<'_>,
    prefix: &mut String,
    out: &mut std::collections::HashMap<String, gix::ObjectId>,
) {
    for entry_ref in tree.iter() {
        let entry = match entry_ref {
            Ok(e) => e,
            Err(_) => continue,
        };

        let name = entry.filename().to_string();
        let full_path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };

        let mode = entry.mode();
        if mode.is_tree() {
            if let Ok(obj) = entry.object() {
                let subtree = obj.into_tree();
                let mut new_prefix = full_path;
                collect_tree_entries(_repo, &subtree, &mut new_prefix, out);
            }
        } else {
            out.insert(full_path, entry.oid().into());
        }
    }
}
