use std::fs;
use zed_extension_api::{
    self as zed, Architecture, DownloadedFileType, GithubReleaseOptions, Os, Result,
    settings::LspSettings,
};

const SERVER_NAME: &str = "protide-lsp";
const GITHUB_REPO: &str = "dreygur/protide";

struct ProtideExtension {
    cached_binary_path: Option<String>,
}

impl ProtideExtension {
    fn language_server_binary(&mut self, worktree: &zed::Worktree) -> Result<zed::Command> {
        let binary_settings = LspSettings::for_worktree(SERVER_NAME, worktree)
            .ok()
            .and_then(|s| s.binary);

        let args = binary_settings
            .as_ref()
            .and_then(|s| s.arguments.clone())
            .unwrap_or_default();

        let env = worktree.shell_env();

        // 1. User-configured path
        if let Some(path) = binary_settings.and_then(|s| s.path) {
            return Ok(zed::Command { command: path, args, env });
        }

        // 2. Cached path from previous download
        if let Some(path) = &self.cached_binary_path {
            if fs::metadata(path).is_ok() {
                return Ok(zed::Command { command: path.clone(), args, env });
            }
        }

        // 3. Binary already on PATH
        if let Some(path) = worktree.which(SERVER_NAME) {
            self.cached_binary_path = Some(path.clone());
            return Ok(zed::Command { command: path, args, env });
        }

        // 4. Download from GitHub releases
        let release = zed::latest_github_release(
            GITHUB_REPO,
            GithubReleaseOptions { require_assets: true, pre_release: false },
        )?;

        let (os, arch) = zed::current_platform();
        let target = match (os, arch) {
            (Os::Mac, Architecture::Aarch64) => "aarch64-apple-darwin",
            (Os::Mac, Architecture::X8664) => "x86_64-apple-darwin",
            (Os::Linux, Architecture::Aarch64) => "aarch64-unknown-linux-gnu",
            (Os::Linux, Architecture::X8664) => "x86_64-unknown-linux-gnu",
            (Os::Windows, Architecture::X8664) => "x86_64-pc-windows-msvc",
            _ => return Err(format!("unsupported platform: {os:?} {arch:?}")),
        };

        let asset_name = format!("protide-lsp-{target}.tar.gz");
        let file_type = if os == Os::Windows {
            DownloadedFileType::Zip
        } else {
            DownloadedFileType::GzipTar
        };

        let asset = release
            .assets
            .into_iter()
            .find(|a| a.name == asset_name)
            .ok_or_else(|| format!("no release asset named {asset_name}"))?;

        let version_dir = format!("{SERVER_NAME}-{}", release.version);
        let binary_path = format!("{version_dir}/{SERVER_NAME}");

        zed::download_file(&asset.download_url, &version_dir, file_type)?;
        zed::make_file_executable(&binary_path)?;
        remove_outdated_versions(&version_dir)?;

        self.cached_binary_path = Some(binary_path.clone());
        Ok(zed::Command { command: binary_path, args, env })
    }
}

fn remove_outdated_versions(current_dir: &str) -> Result<()> {
    let entries = fs::read_dir(".").map_err(|e| format!("failed to read dir: {e}"))?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(SERVER_NAME) && name != current_dir {
            fs::remove_dir_all(entry.path()).ok();
        }
    }
    Ok(())
}

impl zed::Extension for ProtideExtension {
    fn new() -> Self {
        Self { cached_binary_path: None }
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        self.language_server_binary(worktree)
    }

    fn language_server_initialization_options(
        &mut self,
        server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        LspSettings::for_worktree(server_id.as_ref(), worktree)
            .map(|s| s.initialization_options)
    }

    fn language_server_workspace_configuration(
        &mut self,
        server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        LspSettings::for_worktree(server_id.as_ref(), worktree)
            .map(|s| s.settings)
    }
}

zed::register_extension!(ProtideExtension);
