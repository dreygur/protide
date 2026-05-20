//! Remote operations: fetch, pull, push, connect remote.

use super::GitRepo;

impl GitRepo {
    /// Add or update a named remote.
    pub fn connect_remote(&self, name: &str, url: &str) -> Result<(), String> {
        // If remote already exists, update its URL; otherwise create it.
        match self.repo.find_remote(name) {
            Ok(_) => self.repo.remote_set_url(name, url)
                .map_err(|e| format!("Set remote URL failed: {}", e)),
            Err(_) => self.repo.remote(name, url)
                .map(|_| ())
                .map_err(|e| format!("Add remote failed: {}", e)),
        }
    }

    /// List remotes as (name, url) pairs.
    pub fn list_remotes(&self) -> Result<Vec<(String, String)>, String> {
        let names = self.repo.remotes()
            .map_err(|e| format!("List remotes failed: {}", e))?;
        let mut result = Vec::new();
        for name in names.iter().flatten() {
            if let Ok(remote) = self.repo.find_remote(name) {
                let url = remote.url().unwrap_or("").to_string();
                result.push((name.to_string(), url));
            }
        }
        Ok(result)
    }

    /// Fetch from a remote (download objects, don't merge).
    pub fn fetch(&self, remote_name: &str) -> Result<(), String> {
        let mut remote = self.repo.find_remote(remote_name)
            .map_err(|e| format!("Remote '{}' not found: {}", remote_name, e))?;

        let mut opts = git2::FetchOptions::new();
        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(default_credentials);
        opts.remote_callbacks(callbacks);

        remote.fetch::<&str>(&[], Some(&mut opts), None)
            .map_err(|e| format!("Fetch failed: {}", e))
    }

    /// Pull (fetch + fast-forward merge) the current branch from `remote_name`.
    pub fn pull(&self, remote_name: &str) -> Result<(), String> {
        self.fetch(remote_name)?;

        // Determine the tracking branch
        let head = self.repo.head().map_err(|e| format!("HEAD error: {}", e))?;
        let branch_name = head.shorthand().ok_or("Detached HEAD")?.to_string();
        let remote_ref = format!("{}/{}", remote_name, branch_name);

        let fetch_head = self.repo.find_reference("FETCH_HEAD")
            .or_else(|_| self.repo.find_reference(&format!("refs/remotes/{}", remote_ref)))
            .map_err(|e| format!("Fetch HEAD not found: {}", e))?;

        let ann = self.repo.reference_to_annotated_commit(&fetch_head)
            .map_err(|e| format!("Annotated commit error: {}", e))?;

        let (analysis, _) = self.repo.merge_analysis(&[&ann])
            .map_err(|e| format!("Merge analysis failed: {}", e))?;

        if analysis.is_up_to_date() {
            return Ok(());
        }
        if !analysis.is_fast_forward() {
            return Err("Remote has diverged; manual merge required.".to_string());
        }

        let target = fetch_head.target().ok_or("No target")?;
        let head_ref = self.repo.head().map_err(|e| e.to_string())?;
        let resolved = head_ref.resolve().map_err(|e| e.to_string())?;
        let mut resolved_ref = self.repo.find_reference(resolved.name().ok_or("No ref name")?)
            .map_err(|e| e.to_string())?;
        resolved_ref.set_target(target, "pull fast-forward")
            .map_err(|e| format!("Set target failed: {}", e))?;
        self.repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
            .map_err(|e| format!("Checkout failed: {}", e))
    }

    /// Push the current branch to `remote_name`.
    pub fn push(&self, remote_name: &str) -> Result<(), String> {
        let mut remote = self.repo.find_remote(remote_name)
            .map_err(|e| format!("Remote '{}' not found: {}", remote_name, e))?;

        let head = self.repo.head().map_err(|e| format!("HEAD error: {}", e))?;
        let branch = head.shorthand().ok_or("Detached HEAD")?;
        let refspec = format!("refs/heads/{0}:refs/heads/{0}", branch);

        let mut opts = git2::PushOptions::new();
        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(default_credentials);
        opts.remote_callbacks(callbacks);

        remote.push(&[refspec.as_str()], Some(&mut opts))
            .map_err(|e| format!("Push failed: {}", e))
    }
}

/// Default credential callback - uses SSH agent or git credential helper via libgit2.
fn default_credentials(
    url: &str,
    username: Option<&str>,
    allowed: git2::CredentialType,
) -> Result<git2::Cred, git2::Error> {
    if allowed.contains(git2::CredentialType::SSH_KEY) {
        let user = username.unwrap_or("git");
        return git2::Cred::ssh_key_from_agent(user);
    }
    if allowed.contains(git2::CredentialType::DEFAULT) {
        return git2::Cred::default();
    }
    Err(git2::Error::from_str(&format!("No credentials available for {}", url)))
}
