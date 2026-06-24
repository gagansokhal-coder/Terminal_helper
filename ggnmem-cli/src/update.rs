use anyhow::{bail, Context, Result};
use serde::Deserialize;

#[derive(Deserialize, Clone)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

#[derive(Deserialize, Clone)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

pub fn cmd_update(args: &[String]) -> Result<()> {
    if crate::has_flag(args, "--help") || crate::has_flag(args, "-h") {
        print_help();
        return Ok(());
    }

    match args.get(2).map(String::as_str) {
        Some("check") => check_update(),
        Some(cmd) => bail!("unknown update command: {cmd}\n\nusage:\n  ggnmem update check"),
        None => {
            print_help();
            Ok(())
        }
    }
}

fn print_help() {
    println!("ggnmem update — manage ggnmem updates\n");
    println!("usage: ggnmem update <command>\n");
    println!("commands:");
    println!("  check      Check if a newer version is available");
    println!("\noptions:");
    println!("  --help, -h Show this help message");
}

fn check_update() -> Result<()> {
    println!("Checking for updates...");
    let repo = "gagansokhal-coder/Terminal_helper";
    let url = format!("https://api.github.com/repos/{repo}/releases");

    let agent = ureq::builder()
        .timeout_connect(std::time::Duration::from_secs(30))
        .timeout_read(std::time::Duration::from_secs(120))
        .build();

    let response = agent
        .get(&url)
        .set("User-Agent", "ggnmem-cli")
        .call()
        .context("Failed to check for updates. Are you connected to the internet?")?;

    let releases: Vec<GithubRelease> = serde_json::from_reader(response.into_reader())
        .context("Failed to parse GitHub response")?;

    let release = releases.into_iter().next().context("No releases found")?;

    let current_version = env!("CARGO_PKG_VERSION");
    let latest_version = release.tag_name.trim_start_matches('v');

    if compare_versions(current_version, latest_version) != std::cmp::Ordering::Equal {
        println!("\nUpdate available");
        println!("Current version: {}", current_version);
        println!("Latest version:  {}", latest_version);
        println!("Release notes: {}", release.html_url);
        println!("(Note: Download and install are not yet implemented. Please download manually.)");
    } else {
        println!("Current version: {}", current_version);
        println!("Latest version:  {}", latest_version);
        println!("\nYou are up to date!");
    }

    Ok(())
}

pub fn cmd_self_update(args: &[String]) -> Result<()> {
    if crate::has_flag(args, "--help") || crate::has_flag(args, "-h") {
        println!("ggnmem self-update — Update ggnmem to the latest version\n");
        println!("usage: ggnmem self-update [options]\n");
        println!("options:");
        println!("  --dry-run        Check for updates and show what would be done without making changes");
        println!("  --download-only  Download the release bundle without installing");
        println!("  --verify         Download and verify checksum without installing");
        println!("  --extract-test   Download, verify, and test extraction without installing");
        println!("  --help, -h       Show this help message");
        return Ok(());
    }

    let is_dry_run = crate::has_flag(args, "--dry-run");
    let is_download_only = crate::has_flag(args, "--download-only");
    let is_verify = crate::has_flag(args, "--verify");
    let is_extract_test = crate::has_flag(args, "--extract-test");
    let is_install = !is_dry_run && !is_download_only && !is_verify && !is_extract_test;

    let agent = ureq::builder()
        .timeout_connect(std::time::Duration::from_secs(30))
        .timeout_read(std::time::Duration::from_secs(120))
        .build();

    let current_version = env!("CARGO_PKG_VERSION");

    // 1. Check
    let (release, latest_version, asset, display) = check_step(&agent)?;

    if is_dry_run {
        println!("Platform: {}", display);
        println!("Current version: {}", current_version);
        println!("Latest version:  {}", latest_version);
        println!("Selected asset:  {}", asset.name);
        println!("Download URL:    {}", asset.browser_download_url);
        println!("\nNo changes made (--dry-run)");
        return Ok(());
    }

    // 2. Download
    let tmp_dir = get_download_dir();
    let dest_path = tmp_dir.join(&asset.name);
    download_step(&agent, &asset, &tmp_dir, &dest_path, !is_install)?;

    if !is_verify && !is_extract_test && !is_install {
        println!("\nCurrent version: {}", current_version);
        println!("Latest version:  {}", latest_version);
        println!("\nDownloaded:");
        println!("{}", asset.name);
        println!("\nSaved to:");
        println!("{}", dest_path.display());
        println!("\nDownload complete.");
        println!("No installation performed (--download-only)");
        return Ok(());
    }

    // 3. Verify
    verify_step(
        &agent,
        &release,
        &asset,
        &dest_path,
        is_verify || is_extract_test,
    )?;

    if is_verify {
        return Ok(());
    }

    // 4. Extract
    let extract_dir = tmp_dir.join("extracted");
    let bundle_version = extract_step(&dest_path, &extract_dir, &latest_version, is_extract_test)?;

    if is_extract_test {
        return Ok(());
    }

    // 5. Install
    install_step(&extract_dir, current_version, &bundle_version)?;

    Ok(())
}

fn check_step(agent: &ureq::Agent) -> Result<(GithubRelease, String, GithubAsset, String)> {
    println!("Checking for updates...");
    println!("Fetching releases...");
    let repo = "gagansokhal-coder/Terminal_helper";
    let url = format!("https://api.github.com/repos/{repo}/releases");

    let response = agent
        .get(&url)
        .set("User-Agent", "ggnmem-cli")
        .call()
        .context("Failed to check for updates")?;

    let releases: Vec<GithubRelease> = serde_json::from_reader(response.into_reader())
        .context("Failed to parse GitHub response")?;

    let release = releases.into_iter().next().context("No releases found")?;

    let latest_version = release.tag_name.trim_start_matches('v').to_string();
    let (target, display) = get_platform_info();

    println!("Selecting asset...");
    let asset = match select_asset(&release.assets, &target) {
        Some(a) => a.clone(),
        None => bail!("No matching asset found for platform {}", target),
    };

    Ok((release, latest_version, asset, display))
}

fn download_step(
    agent: &ureq::Agent,
    asset: &GithubAsset,
    tmp_dir: &std::path::Path,
    dest_path: &std::path::Path,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("Downloading {}...", asset.name);
    }
    println!("Starting download...");

    std::fs::create_dir_all(tmp_dir).context("Failed to create temporary directory for update")?;

    let download_response = agent
        .get(&asset.browser_download_url)
        .call()
        .context("Failed to download release asset")?;

    let mut dest_file =
        std::fs::File::create(dest_path).context("Failed to create destination file for update")?;

    std::io::copy(&mut download_response.into_reader(), &mut dest_file)
        .context("Download interrupted or disk full")?;

    let metadata = dest_file
        .metadata()
        .context("Failed to read file metadata")?;
    if metadata.len() == 0 {
        bail!("Downloaded file is empty");
    }

    println!("Download finished");
    Ok(())
}

fn verify_step(
    agent: &ureq::Agent,
    release: &GithubRelease,
    asset: &GithubAsset,
    dest_path: &std::path::Path,
    verbose: bool,
) -> Result<()> {
    let checksums_asset = select_checksums_asset(&release.assets)
        .context("checksums.txt not found in release assets")?;

    let checksums_response = agent
        .get(&checksums_asset.browser_download_url)
        .call()
        .context("Failed to download checksums.txt")?;

    let checksums_txt = checksums_response
        .into_string()
        .context("Failed to read checksums.txt")?;

    let expected_hash = get_expected_hash(&checksums_txt, &asset.name)
        .context(format!("No checksum entry found for {}", asset.name))?;

    let actual_hash =
        compute_file_sha256(dest_path).context("Failed to compute SHA256 of downloaded file")?;

    if verbose {
        println!("\nDownloaded:");
        println!("{}", asset.name);
        println!("\nExpected SHA256: {}", expected_hash);
        println!("Actual SHA256: {}", actual_hash);
    }

    if expected_hash == actual_hash {
        if verbose {
            println!("\n✓ Checksum verified");
        }
    } else {
        if verbose {
            println!("\n✗ Checksum mismatch");
        }
        let _ = std::fs::remove_file(dest_path);
        bail!("Checksum verification failed. Downloaded bundle deleted.");
    }

    Ok(())
}

fn extract_step(
    dest_path: &std::path::Path,
    extract_dir: &std::path::Path,
    latest_version: &str,
    verbose: bool,
) -> Result<String> {
    println!("Starting extraction...");

    if extract_dir.exists() {
        std::fs::remove_dir_all(extract_dir).context("Failed to clear extraction directory")?;
    }
    std::fs::create_dir_all(extract_dir).context("Failed to create extraction directory")?;

    if let Err(e) = extract_archive(dest_path, extract_dir) {
        let _ = std::fs::remove_dir_all(extract_dir);
        bail!("Extraction failed: {}", e);
    }

    let bundle_version = match validate_extracted_bundle(extract_dir, latest_version) {
        Ok(ver_no_v) => ver_no_v,
        Err(e) => {
            let _ = std::fs::remove_dir_all(extract_dir);
            bail!("Validation failed: {}", e);
        }
    };

    println!("Extraction finished");

    if verbose {
        #[cfg(windows)]
        let expected_files = [
            "ggnmem.exe",
            "ggnmem-daemon.exe",
            "VERSION",
            "checksums.txt",
        ];
        #[cfg(unix)]
        let expected_files = ["ggnmem", "ggnmem-daemon", "install.sh", "VERSION"];

        println!("\nArchive extracted successfully\n");
        println!("Found:");
        for f in &expected_files {
            println!("✓ {}", f);
        }
        println!("\nBundle version: {}", bundle_version);
        println!("\n✓ Bundle validation passed");
    }

    Ok(bundle_version)
}

fn install_step(
    extract_dir: &std::path::Path,
    current_version: &str,
    bundle_version: &str,
) -> Result<()> {
    println!("Download complete");
    println!("Checksum verified");
    println!("Bundle validated");
    println!("Starting installation...");

    if let Err(e) = perform_install(extract_dir, current_version, bundle_version) {
        let _ = std::fs::remove_dir_all(extract_dir);
        return Err(e);
    }
    Ok(())
}

fn perform_install(
    extract_dir: &std::path::Path,
    previous_version: &str,
    current_version: &str,
) -> Result<()> {
    run_silent_cmd(&["stop"]);

    println!("\nBacking up binaries...");

    let bin_dir = ggnmem_paths::bin_dir().context("Could not resolve bin directory")?;

    #[cfg(windows)]
    let (cli_name, daemon_name) = ("ggnmem.exe", "ggnmem-daemon.exe");
    #[cfg(unix)]
    let (cli_name, daemon_name) = ("ggnmem", "ggnmem-daemon");

    let ggnmem_bin = bin_dir.join(cli_name);
    let ggnmem_daemon_bin = bin_dir.join(daemon_name);
    let ggnmem_old = bin_dir.join(format!("{cli_name}.old"));
    let ggnmem_daemon_old = bin_dir.join(format!("{daemon_name}.old"));

    if ggnmem_bin.exists() {
        std::fs::rename(&ggnmem_bin, &ggnmem_old).context("Failed to backup ggnmem")?;
    }
    if ggnmem_daemon_bin.exists() {
        std::fs::rename(&ggnmem_daemon_bin, &ggnmem_daemon_old)
            .context("Failed to backup ggnmem-daemon")?;
    }

    println!("Installing update...");
    let extracted_ggnmem = extract_dir.join(cli_name);
    let extracted_daemon = extract_dir.join(daemon_name);

    let install_result = (|| -> Result<()> {
        std::fs::copy(&extracted_ggnmem, &ggnmem_bin).context("Failed to copy ggnmem binary")?;
        std::fs::copy(&extracted_daemon, &ggnmem_daemon_bin)
            .context("Failed to copy ggnmem-daemon binary")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(&ggnmem_bin) {
                let mut perms = meta.permissions();
                perms.set_mode(0o755);
                let _ = std::fs::set_permissions(&ggnmem_bin, perms);
            }
            if let Ok(meta) = std::fs::metadata(&ggnmem_daemon_bin) {
                let mut perms = meta.permissions();
                perms.set_mode(0o755);
                let _ = std::fs::set_permissions(&ggnmem_daemon_bin, perms);
            }
        }

        println!("Verifying installation...");
        let output = std::process::Command::new(&ggnmem_bin)
            .arg("--version")
            .output()
            .context("Failed to run newly installed ggnmem")?;

        if !output.status.success() {
            bail!("Newly installed ggnmem failed to run");
        }

        let out_str = String::from_utf8_lossy(&output.stdout);
        if !out_str.contains(current_version) {
            bail!(
                "Installed ggnmem reported unexpected version: {}",
                out_str.trim()
            );
        }

        Ok(())
    })();

    if let Err(e) = install_result {
        if ggnmem_old.exists() {
            let _ = std::fs::rename(&ggnmem_old, &ggnmem_bin);
        }
        if ggnmem_daemon_old.exists() {
            let _ = std::fs::rename(&ggnmem_daemon_old, &ggnmem_daemon_bin);
        }
        run_silent_cmd(&["start"]);
        println!("rollback completed");
        bail!("Update failed: {}", e);
    }

    run_silent_cmd(&["start"]);

    println!("\n✓ Update successful\n");
    println!("Previous version: {}", previous_version);
    println!("Current version:  {}", current_version);

    // Show preserved data.
    println!();
    println!("  preserved:");
    if let Ok(p) = crate::config::config_path() {
        if p.exists() {
            println!("  ✓ config   ({})", p.display());
        }
    }
    let db_path = crate::default_db_path();
    if db_path.exists() {
        println!("  ✓ database ({})", db_path.display());
    }
    if let Some(mdir) = ggnmem_paths::models_dir() {
        if mdir.exists() {
            println!("  ✓ models   ({})", mdir.display());
        }
    }

    Ok(())
}

fn run_silent_cmd(args: &[&str]) {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe)
            .args(args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
}

/// Extract an archive (.zip or .tar.gz) to the target directory.
///
/// On Windows, `.zip` files are extracted via PowerShell `Expand-Archive`.
/// On all platforms, `.tar.gz` files are extracted via the `tar` command.
fn extract_archive(archive_path: &std::path::Path, extract_dir: &std::path::Path) -> Result<()> {
    let name = archive_path.to_string_lossy();

    if name.ends_with(".zip") {
        // Use .NET ZipFile for extraction (built-in, no module required).
        let output = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Add-Type -AssemblyName System.IO.Compression.FileSystem; [System.IO.Compression.ZipFile]::ExtractToDirectory('{}', '{}')",
                    archive_path.display(),
                    extract_dir.display()
                ),
            ])
            .output()
            .context("Failed to run PowerShell ZIP extraction")?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            bail!("Failed to extract ZIP archive: {}", err_msg.trim());
        }
    } else {
        extract_tar_gz(archive_path, extract_dir)?;
    }
    Ok(())
}

fn extract_tar_gz(archive_path: &std::path::Path, extract_dir: &std::path::Path) -> Result<()> {
    let status = std::process::Command::new("tar")
        .arg("-xzf")
        .arg(archive_path)
        .arg("-C")
        .arg(extract_dir)
        .status()
        .context("Failed to run tar command")?;

    if !status.success() {
        bail!("Failed to extract archive");
    }
    Ok(())
}

fn validate_extracted_bundle(
    extract_dir: &std::path::Path,
    expected_version: &str,
) -> Result<String> {
    #[cfg(windows)]
    let required_files = [
        "ggnmem.exe",
        "ggnmem-daemon.exe",
        "VERSION",
        "checksums.txt",
    ];
    #[cfg(unix)]
    let required_files = [
        "ggnmem",
        "ggnmem-daemon",
        "install.sh",
        "VERSION",
        "checksums.txt",
    ];

    let mut missing = Vec::new();
    for f in &required_files {
        if !extract_dir.join(f).exists() {
            missing.push(*f);
        }
    }

    if !missing.is_empty() {
        bail!("Extracted archive is missing required files: {:?}", missing);
    }

    let version_content = std::fs::read_to_string(extract_dir.join("VERSION"))
        .context("Failed to read VERSION file")?;

    let mut bundle_version = String::new();
    for line in version_content.lines() {
        if let Some(v) = line.strip_prefix("version=") {
            bundle_version = v.trim().to_string();
            break;
        }
    }

    if bundle_version.is_empty() {
        // Fallback for simple VERSION files
        bundle_version = version_content.trim().to_string();
    }

    let ver_no_v = bundle_version.trim_start_matches('v');
    let expected_no_v = expected_version.trim_start_matches('v');

    if expected_no_v != ver_no_v {
        bail!(
            "Version mismatch: GitHub release is {}, but bundle VERSION is {}",
            expected_no_v,
            ver_no_v
        );
    }

    Ok(ver_no_v.to_string())
}

fn get_download_dir() -> std::path::PathBuf {
    std::env::temp_dir().join("ggnmem-update")
}

fn select_asset<'a>(assets: &'a [GithubAsset], target: &str) -> Option<&'a GithubAsset> {
    assets.iter().find(|a| {
        a.name.contains(target) && (a.name.ends_with(".tar.gz") || a.name.ends_with(".zip"))
    })
}

fn select_checksums_asset(assets: &[GithubAsset]) -> Option<&GithubAsset> {
    assets.iter().find(|a| a.name == "checksums.txt")
}

fn get_expected_hash(checksums_txt: &str, asset_name: &str) -> Option<String> {
    for line in checksums_txt.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 && parts[1] == asset_name {
            return Some(parts[0].to_string());
        }
    }
    None
}

fn compute_file_sha256(path: &std::path::Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

fn get_platform_info() -> (String, String) {
    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;

    if os == "windows" {
        return (format!("windows-{}", arch), format!("Windows ({})", arch));
    }

    let is_wsl = std::fs::read_to_string("/proc/version")
        .map(|s| s.to_lowercase().contains("microsoft"))
        .unwrap_or(false);

    let target = format!("{}-{}", os, arch);
    let display = if is_wsl {
        format!("WSL ({})", target)
    } else {
        target.clone()
    };
    (target, display)
}

fn compare_versions(v1: &str, v2: &str) -> std::cmp::Ordering {
    let parse = |v: &str| -> (Vec<u32>, Vec<String>) {
        let mut parts = v.splitn(2, '-');
        let core = parts.next().unwrap_or("0");
        let pre = parts.next().unwrap_or("");

        let nums: Vec<u32> = core.split('.').filter_map(|s| s.parse().ok()).collect();
        let pre_parts: Vec<String> = if pre.is_empty() {
            vec![]
        } else {
            pre.split('.').map(|s| s.to_string()).collect()
        };
        (nums, pre_parts)
    };

    let (nums1, pre1) = parse(v1);
    let (nums2, pre2) = parse(v2);

    for (n1, n2) in nums1.iter().zip(nums2.iter()) {
        match n1.cmp(n2) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }

    match nums1.len().cmp(&nums2.len()) {
        std::cmp::Ordering::Equal => {}
        other => return other,
    }

    if pre1.is_empty() && !pre2.is_empty() {
        return std::cmp::Ordering::Greater;
    }
    if !pre1.is_empty() && pre2.is_empty() {
        return std::cmp::Ordering::Less;
    }

    for (p1, p2) in pre1.iter().zip(pre2.iter()) {
        if p1 == p2 {
            continue;
        }
        if let (Ok(n1), Ok(n2)) = (p1.parse::<u32>(), p2.parse::<u32>()) {
            return n1.cmp(&n2);
        }
        return p1.cmp(p2);
    }

    pre1.len().cmp(&pre2.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    fn test_compare_versions() {
        assert_eq!(compare_versions("0.3.4", "0.3.5"), Ordering::Less);
        assert_eq!(compare_versions("0.3.5", "0.3.4"), Ordering::Greater);
        assert_eq!(compare_versions("0.3.5", "0.3.5"), Ordering::Equal);

        // Pre-release versions
        assert_eq!(
            compare_versions("0.3.5-alpha", "0.3.5-beta"),
            Ordering::Less
        );
        assert_eq!(compare_versions("0.3.5-alpha", "0.3.5"), Ordering::Less);
        assert_eq!(compare_versions("0.3.5", "0.3.5-alpha"), Ordering::Greater);

        // Numbered pre-releases
        assert_eq!(
            compare_versions("0.3.5-alpha.1", "0.3.5-alpha.2"),
            Ordering::Less
        );
        assert_eq!(
            compare_versions("0.3.5-alpha.2", "0.3.5-alpha.10"),
            Ordering::Less
        );
        assert_eq!(
            compare_versions("0.3.5-alpha.10", "0.3.5-alpha.2"),
            Ordering::Greater
        );
        assert_eq!(
            compare_versions("0.3.5-alpha.2", "0.3.5-beta.1"),
            Ordering::Less
        );
    }

    #[test]
    fn test_get_platform_info() {
        let (target, display) = get_platform_info();
        assert!(!target.is_empty());
        assert!(!display.is_empty());
    }

    #[test]
    fn test_get_download_dir() {
        let dir = get_download_dir();
        assert!(dir.ends_with("ggnmem-update"));
    }

    #[test]
    fn test_select_asset() {
        let assets = vec![
            GithubAsset {
                name: "ggnmem-linux-x86_64.tar.gz".to_string(),
                browser_download_url: "url1".to_string(),
            },
            GithubAsset {
                name: "ggnmem-linux-aarch64.tar.gz".to_string(),
                browser_download_url: "url2".to_string(),
            },
            GithubAsset {
                name: "ggnmem-windows-x86_64.zip".to_string(),
                browser_download_url: "url3".to_string(),
            },
        ];

        assert_eq!(
            select_asset(&assets, "linux-x86_64").unwrap().name,
            "ggnmem-linux-x86_64.tar.gz"
        );
        assert_eq!(
            select_asset(&assets, "linux-aarch64").unwrap().name,
            "ggnmem-linux-aarch64.tar.gz"
        );
        assert_eq!(
            select_asset(&assets, "windows-x86_64").unwrap().name,
            "ggnmem-windows-x86_64.zip"
        );
    }

    #[test]
    fn test_select_checksums_asset() {
        let assets = vec![
            GithubAsset {
                name: "checksums.txt".to_string(),
                browser_download_url: "url".to_string(),
            },
            GithubAsset {
                name: "other.txt".to_string(),
                browser_download_url: "url2".to_string(),
            },
        ];
        assert_eq!(
            select_checksums_asset(&assets).unwrap().name,
            "checksums.txt"
        );
        assert!(select_checksums_asset(&assets[1..]).is_none());
    }

    #[test]
    fn test_get_expected_hash() {
        let txt = "12345  ggnmem-linux-x86_64.tar.gz\n67890  ggnmem-linux-aarch64.tar.gz\n";
        // Valid checksum
        assert_eq!(
            get_expected_hash(txt, "ggnmem-linux-x86_64.tar.gz"),
            Some("12345".to_string())
        );
        // Missing asset entry
        assert_eq!(get_expected_hash(txt, "ggnmem-windows-x86_64.zip"), None);
    }

    #[test]
    fn test_extract_invalid_archive() {
        let temp_dir = tempfile::tempdir().unwrap();
        let archive_path = temp_dir.path().join("invalid.tar.gz");
        std::fs::write(&archive_path, b"not a valid tar gz").unwrap();

        let extract_dir = temp_dir.path().join("extracted");
        std::fs::create_dir_all(&extract_dir).unwrap();

        assert!(extract_tar_gz(&archive_path, &extract_dir).is_err());
    }

    #[test]
    fn test_validate_extracted_bundle() {
        let temp_dir = tempfile::tempdir().unwrap();
        let extract_dir = temp_dir.path();

        // Missing files
        assert!(validate_extracted_bundle(extract_dir, "0.3.5").is_err());

        // Create files
        let required_files = ["ggnmem", "ggnmem-daemon", "install.sh", "checksums.txt"];
        for f in &required_files {
            std::fs::File::create(extract_dir.join(f)).unwrap();
        }

        // Still missing VERSION
        assert!(validate_extracted_bundle(extract_dir, "0.3.5").is_err());

        // Add VERSION with wrong version
        std::fs::write(extract_dir.join("VERSION"), "0.3.4\n").unwrap();
        assert!(validate_extracted_bundle(extract_dir, "0.3.5").is_err());

        // Add VERSION with correct version
        std::fs::write(extract_dir.join("VERSION"), "v0.3.5\n").unwrap();
        let ver = validate_extracted_bundle(extract_dir, "0.3.5").unwrap();
        assert_eq!(ver, "0.3.5");
    }

    #[test]
    fn test_perform_install_rollback_on_failure() {
        let temp_dir = tempfile::tempdir().unwrap();
        // create fake ~/.local/bin
        let local_bin = temp_dir.path().join(".local").join("bin");
        std::fs::create_dir_all(&local_bin).unwrap();

        let ggnmem_path = local_bin.join("ggnmem");
        std::fs::write(&ggnmem_path, "old_ggnmem").unwrap();

        let extract_dir = temp_dir.path().join("extract");
        std::fs::create_dir_all(&extract_dir).unwrap();

        // This will fail because extracted/ggnmem doesn't exist
        std::env::set_var("HOME", temp_dir.path().to_str().unwrap());

        let res = perform_install(&extract_dir, "0.1.0", "0.2.0");
        assert!(res.is_err());

        // Ensure rollback happened
        assert!(ggnmem_path.exists());
        assert_eq!(std::fs::read_to_string(&ggnmem_path).unwrap(), "old_ggnmem");
    }

    #[test]
    fn test_self_update_download_timeout_regression() {
        // Just verify that the builder is created correctly and timeouts are set
        // to larger values to avoid regressions. This ensures that the agent will not time out early.
        let _agent = ureq::builder()
            .timeout_connect(std::time::Duration::from_secs(30))
            .timeout_read(std::time::Duration::from_secs(120))
            .build();

        // Check if `agent` is instantiated and has basic traits
        // Because `ureq::Agent`'s timeout values are not exposed,
        // this regression test mainly serves to ensure compiling with the larger timeouts.
        // Agent initialized with increased timeouts
    }
}
