//! Functions for calling BI's binarize.exe (on Windows, or under Proton on Linux)

use std::env::var;
#[cfg(windows)]
use std::env::temp_dir;
use std::fs::File;
#[cfg(windows)]
use std::fs::{create_dir_all, remove_dir_all};
use std::io::{Write, Cursor, Error};
#[cfg(windows)]
use std::io::Read;
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(windows)]
use winreg::RegKey;
#[cfg(windows)]
use winreg::enums::*;

use crate::*;
use crate::error::*;

#[cfg(windows)]
fn find_binarize_exe() -> Result<PathBuf, Error> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let binarize = hkcu.open_subkey("Software\\Bohemia Interactive\\binarize")?;
    let value: String = binarize.get_value("path")?;

    Ok(PathBuf::from(value).join("binarize_x64.exe"))
}

static PROTON_ENABLED: AtomicBool = AtomicBool::new(false);

/// Opt in to binarizing models with binarize.exe under Proton (unix only, see `--proton-binarize`).
pub fn set_proton(enabled: bool) {
    PROTON_ENABLED.store(enabled, Ordering::SeqCst);
}

/// Whether models can be binarized on this system with the current settings.
pub fn available() -> bool {
    cfg!(windows) || PROTON_ENABLED.load(Ordering::SeqCst)
}

/// Removes the Proton sandbox, if one was created. Does nothing on Windows.
pub fn cleanup() {
    #[cfg(unix)]
    proton::cleanup();
}

#[cfg(windows)]
fn create_temp_directory(name: &str) -> Result<PathBuf, Error> {
    let dir = temp_dir();
    let mut i = 0;

    let mut path;
    loop {
        path = dir.join(format!("armake_{}_{}", name, i));
        if !path.exists() { break; }

        i += 1;
    }

    create_dir_all(&path)?;

    Ok(path)
}

/// Binarizes the given path with BI's binarize.exe (Windows, or Linux with `--proton-binarize`).
///
/// `addon_root` is the folder the addon is built from and `prefix` its PBO prefix: binarize.exe
/// resolves texture references (`prefix\data\x.paa`) relative to its current directory, so the
/// addon is made available at `P:\<prefix>`.
#[cfg(unix)]
pub fn binarize(input: &PathBuf, addon_root: &Path, prefix: &str) -> Result<Cursor<Box<[u8]>>, Error> {
    if !PROTON_ENABLED.load(Ordering::SeqCst) {
        return Err(error!("binarize.exe is only available on Windows, or on Linux through Proton. Use --proton-binarize (see README) or rapify to binarize configs."));
    }

    proton::binarize(input, addon_root, prefix)
}

/// Binarizes the given path with BI's binarize.exe (Windows, or Linux with `--proton-binarize`).
#[cfg(windows)]
pub fn binarize(input: &PathBuf, _addon_root: &Path, _prefix: &str) -> Result<Cursor<Box<[u8]>>, Error> {
    let binarize_exe = find_binarize_exe().prepend_error("Failed to find BI's binarize.exe:")?;
    if !binarize_exe.exists() {
        return Err(error!("BI's binarize.exe found in registry, but doesn't exist."));
    }

    let input_dir = PathBuf::from(input.parent().unwrap());
    let name = input.file_name().unwrap().to_str().unwrap().to_string();
    let tempdir = create_temp_directory(&name).prepend_error("Failed to create tempfolder:")?;

    let piped = var("BIOUTPUT").unwrap_or_else(|_| "0".to_string()) == "1";

    let mut command = Command::new(binarize_exe);
    // binarize.exe looks up textures relative to its current directory (the P: drive root)
    if Path::new("P:\\").is_dir() {
        command.current_dir("P:\\");
    }
    let binarize_output = command
        .args(&["-norecurse", "-always", "-silent", "-maxProcesses=0", input_dir.to_str().unwrap(), tempdir.to_str().unwrap(), input.file_name().unwrap().to_str().unwrap()])
        .stdout(if piped { Stdio::inherit() } else { Stdio::null() })
        .stderr(if piped { Stdio::inherit() } else { Stdio::null() })
        .output().unwrap();

    if !binarize_output.status.success() {
        let msg = match binarize_output.status.code() {
            Some(code) => format!("binarize.exe terminated with exit code: {}", code),
            None => "binarize.exe terminated by signal.".to_string()
        };
        let outputhint = if !piped { "\nUse BIOUTPUT=1 to see binarize.exe's output." } else { "" };

        return Err(error!("{}{}", msg, outputhint));
    }

    let result_path = tempdir.join(input.strip_prefix(&input_dir).unwrap());
    let mut buffer: Vec<u8> = Vec::new();

    {
        let mut file = File::open(result_path).prepend_error("Failed to open binarize.exe output:")?;
        file.read_to_end(&mut buffer).prepend_error("Failed to read binarize.exe output:")?;
    }

    remove_dir_all(&tempdir).prepend_error("Failed to remove temp directory:")?;

    Ok(Cursor::new(buffer.into_boxed_slice()))
}

/// Binarizes the given path using BI's binarize.exe and writes it to the output.
pub fn cmd_binarize(input: PathBuf, output: PathBuf, force: bool) -> Result<(), Error> {
    if !force && output.exists() {
        return Err(error!("Target file \"{}\" already exists. Use --force to overwrite.", output.display()));
    }

    // on unix, `armake2 binarize` always goes through Proton
    set_proton(true);
    let addon_root = input.parent().map(|p| if p.as_os_str().is_empty() { PathBuf::from(".") } else { p.to_path_buf() }).unwrap_or_else(|| PathBuf::from("."));
    let prefix = addon_root.canonicalize().ok().and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned())).unwrap_or_default();
    let cursor = binarize(&input, &addon_root, &prefix)?;
    let mut file = File::create(&output).prepend_error("Failed to open output:")?;
    file.write_all(cursor.get_ref()).prepend_error("Failed to write result to file:")?;

    Ok(())
}

#[cfg(unix)]
mod proton {
    use std::fs::{File, create_dir_all, read_to_string, remove_dir_all, remove_file};
    use std::io::{Cursor, Error, Read};
    use std::os::unix::fs::symlink;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use std::sync::Mutex;
    use std::thread::sleep;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    use once_cell::sync::Lazy;

    use crate::*;
    use crate::error::*;

    const APP_ID: &str = "830640";
    const PREFIX_TIMEOUT: Duration = Duration::from_secs(300);
    const BINARIZE_TIMEOUT: Duration = Duration::from_secs(600);

    struct Sandbox {
        root: PathBuf,
        proton_dir: PathBuf,
        steam_root: PathBuf,
        counter: u32,
    }

    static SANDBOX: Lazy<Mutex<Option<Sandbox>>> = Lazy::new(|| Mutex::new(None));

    /// The first candidate that has `steamapps/common`.
    pub fn find_steam_root(candidates: &[PathBuf]) -> Option<PathBuf> {
        candidates.iter().find(|c| c.join("steamapps").join("common").is_dir()).cloned()
    }

    fn steam_candidates() -> Vec<PathBuf> {
        let mut c: Vec<PathBuf> = Vec::new();
        if let Some(r) = std::env::var_os("STEAM_ROOT") {
            if !r.is_empty() { c.push(PathBuf::from(r)); }
        }
        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            c.push(home.join(".steam/debian-installation"));
            c.push(home.join(".steam/steam"));
            c.push(home.join(".local/share/Steam"));
        }
        c
    }

    /// `override_dir` is DAYZ_TOOLS; returns the directory holding Bin/Binarize/binarize.exe.
    pub fn find_dayz_tools(steam_root: Option<&Path>, override_dir: Option<PathBuf>) -> Result<PathBuf, Error> {
        let tools = match (override_dir, steam_root) {
            (Some(d), _) => d,
            (None, Some(s)) => s.join("steamapps/common/DayZ Tools"),
            (None, None) => return Err(error!("No Steam installation found (set STEAM_ROOT, or DAYZ_TOOLS to the DayZ Tools folder).")),
        };
        if !tools.join("Bin/Binarize/binarize.exe").is_file() {
            return Err(error!("binarize.exe not found in \"{}\". Install DayZ Tools (Steam app {}) or set DAYZ_TOOLS.", tools.display(), APP_ID));
        }
        Ok(tools)
    }

    /// `override_dir` is PROTON (the Proton directory); returns the directory containing the `proton` script.
    pub fn find_proton(steam_root: Option<&Path>, override_dir: Option<PathBuf>) -> Result<PathBuf, Error> {
        if let Some(d) = override_dir {
            if d.join("proton").is_file() { return Ok(d); }
            return Err(error!("No proton script in PROTON=\"{}\".", d.display()));
        }
        let common = match steam_root {
            Some(s) => s.join("steamapps/common"),
            None => return Err(error!("No Steam installation found (set STEAM_ROOT, or PROTON to a Proton folder).")),
        };
        for name in &["Proton Hotfix", "Proton - Experimental", "Proton Experimental"] {
            if common.join(name).join("proton").is_file() { return Ok(common.join(name)); }
        }
        let mut others: Vec<PathBuf> = match std::fs::read_dir(&common) {
            Ok(rd) => rd.filter_map(|e| e.ok()).map(|e| e.path())
                .filter(|p| p.file_name().and_then(|n| n.to_str()).map_or(false, |n| n.starts_with("Proton")) && p.join("proton").is_file())
                .collect(),
            Err(_) => Vec::new(),
        };
        others.sort();
        others.pop().ok_or_else(|| error!("No Proton found under \"{}\". Install one in Steam or set PROTON.", common.display()))
    }

    /// Windows path (drive Z: is the sandbox root) of a path inside the sandbox.
    pub fn win_path(root: &Path, path: &Path) -> String {
        let rel = path.strip_prefix(root).unwrap_or(path);
        let parts: Vec<&str> = rel.iter().filter_map(|c| c.to_str()).collect();
        format!("Z:\\{}", parts.join("\\"))
    }

    fn copy_dir(src: &Path, dst: &Path) -> Result<(), Error> {
        create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let target = dst.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                copy_dir(&entry.path(), &target)?;
            } else {
                std::fs::copy(entry.path(), &target)?;
            }
        }
        Ok(())
    }

    /// Textures outside the addon (e.g. vanilla `dz\\...`) can't be resolved; their alpha flags stay unset.
    fn warn_missing_textures(log: &Path, model: &str) {
        let text = read_to_string(log).unwrap_or_default();
        for line in text.lines() {
            let lower = line.to_ascii_lowercase();
            if lower.contains("texture") && (lower.contains("cannot") || lower.contains("can't") || lower.contains("not found") || lower.contains("failed")) {
                warning(format!("{}: {}", model, line.trim()), Some("binarize-missing-texture"), (None, None));
            }
        }
    }

    fn tail(path: &Path, lines: usize) -> String {
        let text = read_to_string(path).unwrap_or_default();
        let all: Vec<&str> = text.lines().collect();
        all[all.len().saturating_sub(lines)..].join("\n")
    }

    /// Runs a proton command with output captured in `log`; kills it after `timeout`.
    fn run_proton(sb: &Sandbox, args: &[&str], cwd: &Path, log: &Path, timeout: Duration) -> Result<(), Error> {
        let logfile = File::create(log)?;
        let mut child = Command::new(sb.proton_dir.join("proton"))
            .arg("run").args(args)
            .current_dir(cwd)
            .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", &sb.steam_root)
            .env("STEAM_COMPAT_DATA_PATH", sb.root.join("prefix"))
            .env("SteamAppId", APP_ID)
            .env("SteamGameId", APP_ID)
            .env("STEAM_COMPAT_APP_ID", APP_ID)
            .stdin(Stdio::null())
            .stdout(Stdio::from(logfile.try_clone()?))
            .stderr(Stdio::from(logfile))
            .spawn()
            .prepend_error("Failed to run proton:")?;

        let start = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait()? { break status; }
            if start.elapsed() > timeout {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error!("Timed out after {} s.\n{}", timeout.as_secs(), tail(log, 15)));
            }
            sleep(Duration::from_millis(100));
        };

        if super::var("BIOUTPUT").unwrap_or_else(|_| "0".to_string()) == "1" {
            eprintln!("{}", read_to_string(log).unwrap_or_default());
        }

        if !status.success() {
            let msg = match status.code() {
                Some(code) => format!("proton terminated with exit code: {}", code),
                None => "proton terminated by signal.".to_string(),
            };
            return Err(error!("{}\n{}", msg, tail(log, 15)));
        }
        Ok(())
    }

    fn create_sandbox() -> Result<Sandbox, Error> {
        let steam_root = find_steam_root(&steam_candidates());
        let tools = find_dayz_tools(steam_root.as_deref(), std::env::var_os("DAYZ_TOOLS").filter(|v| !v.is_empty()).map(PathBuf::from))?;
        let proton_dir = find_proton(steam_root.as_deref(), std::env::var_os("PROTON").filter(|v| !v.is_empty()).map(PathBuf::from))?;
        let steam_root = steam_root.unwrap_or_else(|| {
            // only reachable with both overrides set: Proton wants some client install path
            proton_dir.parent().and_then(|p| p.parent()).and_then(|p| p.parent()).map(Path::to_path_buf).unwrap_or_default()
        });

        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0);
        let root = std::env::temp_dir().join(format!("armake2-binarize-{}-{}", std::process::id(), nanos));
        std::fs::create_dir(&root).prepend_error("Failed to create sandbox directory:")?;
        let mut sb = Sandbox { root, proton_dir, steam_root, counter: 0 };

        // from here on the sandbox exists on disk, so a failure must remove it again
        let result = (|| -> Result<(), Error> {
            for d in &["prefix", "in", "out", "P"] {
                create_dir_all(sb.root.join(d))?;
            }
            copy_dir(&tools.join("Bin/Binarize"), &sb.root.join("Binarize")).prepend_error("Failed to copy Binarize:")?;

            eprintln!("Creating a Wine prefix for binarize.exe (the first run takes a while)...");
            let log = sb.root.join("prefix-init.log");
            run_proton(&sb, &["cmd.exe", "/c", "exit"], &sb.root, &log, PREFIX_TIMEOUT)
                .prepend_error("Failed to create the Wine prefix:")?;

            let dosdevices = sb.root.join("prefix/pfx/dosdevices");
            if !dosdevices.is_dir() {
                return Err(error!("Proton did not create a prefix in \"{}\".", sb.root.join("prefix").display()));
            }
            // Wine's default Z: is /, and binarize.exe walks directory trees from the drive root
            let z = dosdevices.join("z:");
            let _ = remove_file(&z);
            symlink(&sb.root, &z).prepend_error("Failed to confine drive Z:")?;
            // P: is the project root; binarize.exe resolves texture paths relative to it
            let p = dosdevices.join("p:");
            let _ = remove_file(&p);
            symlink(sb.root.join("P"), &p).prepend_error("Failed to create drive P:")?;
            Ok(())
        })();

        match result {
            Ok(()) => Ok(sb),
            Err(e) => {
                sb.counter = 0;
                destroy(sb);
                Err(e)
            }
        }
    }

    fn destroy(sb: Sandbox) {
        // wineserver keeps the prefix busy after the last run; stop it before deleting
        let _ = Command::new(sb.proton_dir.join("files/bin/wineserver"))
            .arg("-k")
            .env("WINEPREFIX", sb.root.join("prefix/pfx"))
            .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
            .status();
        sleep(Duration::from_secs(1));
        if let Err(e) = remove_dir_all(&sb.root) {
            eprintln!("Warning: failed to remove \"{}\": {}", sb.root.display(), e);
        }
    }

    pub fn cleanup() {
        let sb = SANDBOX.lock().ok().and_then(|mut g| g.take());
        if let Some(sb) = sb { destroy(sb); }
    }

    /// Makes the addon visible at `<sandbox>/P/<prefix>` (once per addon).
    pub fn stage_addon(root: &Path, prefix: &str, addon_root: &Path) -> Result<(), Error> {
        let mut link = root.join("P");
        for part in prefix.split(|c| c == '\\' || c == '/').filter(|p| !p.is_empty()) {
            link.push(part);
        }
        if link.symlink_metadata().is_ok() {
            return Ok(());
        }
        if let Some(parent) = link.parent() {
            create_dir_all(parent)?;
        }
        symlink(addon_root.canonicalize()?, &link)
    }

    /// Windows path on the P: drive of `relative_dir` inside the addon with the given prefix.
    pub fn p_win_path(prefix: &str, relative_dir: &Path) -> String {
        let mut parts: Vec<String> = prefix.split(|c| c == '\\' || c == '/').filter(|p| !p.is_empty()).map(String::from).collect();
        parts.extend(relative_dir.iter().filter_map(|c| c.to_str()).map(String::from));
        format!("P:\\{}", parts.join("\\"))
    }

    pub fn binarize(input: &PathBuf, addon_root: &Path, prefix: &str) -> Result<Cursor<Box<[u8]>>, Error> {
        let mut guard = SANDBOX.lock().map_err(|_| error!("Binarize sandbox lock poisoned."))?;
        if guard.is_none() {
            *guard = Some(create_sandbox()?);
        }
        let sb = guard.as_mut().unwrap();

        let name = input.file_name().and_then(|n| n.to_str()).ok_or_else(|| error!("Invalid file name \"{}\".", input.display()))?.to_string();
        let relative_dir = input.parent().unwrap_or_else(|| Path::new("")).strip_prefix(addon_root)
            .map_err(|_| error!("\"{}\" is not inside the addon folder \"{}\".", input.display(), addon_root.display()))?;
        stage_addon(&sb.root, prefix, addon_root).prepend_error("Failed to map the addon onto drive P:")?;

        sb.counter += 1;
        let out_dir = sb.root.join("out").join(sb.counter.to_string());
        create_dir_all(&out_dir)?;

        let log = out_dir.join("binarize.log");
        let in_w = p_win_path(prefix, relative_dir);
        let out_w = win_path(&sb.root, &out_dir);
        run_proton(sb, &["Z:\\Binarize\\binarize.exe", "-norecurse", "-always", "-silent", "-maxProcesses=0", &in_w, &out_w, &name],
            &sb.root.join("P"), &log, BINARIZE_TIMEOUT)
            .prepend_error("binarize.exe failed:")?;

        warn_missing_textures(&log, &name);

        let mut buffer: Vec<u8> = Vec::new();
        match File::open(out_dir.join(&name)) {
            Ok(mut f) => { f.read_to_end(&mut buffer).prepend_error("Failed to read binarize.exe output:")?; }
            Err(_) => {}
        }
        if buffer.is_empty() {
            return Err(error!("binarize.exe produced no output for \"{}\" (models referencing textures/materials may need a P: drive).\n{}", name, tail(&log, 15)));
        }

        let _ = remove_dir_all(&out_dir);
        Ok(Cursor::new(buffer.into_boxed_slice()))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        fn touch(p: &Path) {
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, b"").unwrap();
            fs::set_permissions(p, fs::Permissions::from_mode(0o755)).unwrap();
        }

        #[test]
        fn sandbox_layout() {
            let root = tempfile::tempdir().unwrap();
            let src = tempfile::tempdir().unwrap();
            fs::create_dir_all(root.path().join("P")).unwrap();
            stage_addon(root.path(), "my\\mod", src.path()).unwrap();
            let link = root.path().join("P/my/mod");
            assert_eq!(fs::read_link(&link).unwrap(), src.path().canonicalize().unwrap());
            // staging twice is fine
            stage_addon(root.path(), "my\\mod", src.path()).unwrap();
            assert_eq!(p_win_path("my\\mod", Path::new("sub/dir")), "P:\\my\\mod\\sub\\dir");
            assert_eq!(p_win_path("my\\mod", Path::new("")), "P:\\my\\mod");
        }

        #[test]
        fn win_path_is_relative_to_sandbox() {
            let root = Path::new("/tmp/armake2-binarize-1-2");
            assert_eq!(win_path(root, &root.join("in/3")), "Z:\\in\\3");
            assert_eq!(win_path(root, &root.join("Binarize")), "Z:\\Binarize");
        }

        #[test]
        fn steam_root_needs_steamapps_common() {
            let t = tempfile::tempdir().unwrap();
            fs::create_dir_all(t.path().join("b/steamapps/common")).unwrap();
            fs::create_dir_all(t.path().join("a")).unwrap();
            let found = find_steam_root(&[t.path().join("a"), t.path().join("b")]);
            assert_eq!(found, Some(t.path().join("b")));
            assert_eq!(find_steam_root(&[t.path().join("a")]), None);
        }

        #[test]
        fn dayz_tools_discovery() {
            let t = tempfile::tempdir().unwrap();
            assert!(find_dayz_tools(Some(t.path()), None).is_err());
            assert!(find_dayz_tools(None, None).is_err());
            touch(&t.path().join("steamapps/common/DayZ Tools/Bin/Binarize/binarize.exe"));
            assert_eq!(find_dayz_tools(Some(t.path()), None).unwrap(), t.path().join("steamapps/common/DayZ Tools"));
            let other = t.path().join("other");
            assert!(find_dayz_tools(Some(t.path()), Some(other.clone())).is_err());
            touch(&other.join("Bin/Binarize/binarize.exe"));
            assert_eq!(find_dayz_tools(None, Some(other.clone())).unwrap(), other);
        }

        #[test]
        fn proton_discovery_prefers_hotfix_then_experimental_then_any() {
            let t = tempfile::tempdir().unwrap();
            let common = t.path().join("steamapps/common");
            assert!(find_proton(Some(t.path()), None).is_err());
            touch(&common.join("Proton 9.0/proton"));
            assert_eq!(find_proton(Some(t.path()), None).unwrap(), common.join("Proton 9.0"));
            touch(&common.join("Proton - Experimental/proton"));
            assert_eq!(find_proton(Some(t.path()), None).unwrap(), common.join("Proton - Experimental"));
            touch(&common.join("Proton Hotfix/proton"));
            assert_eq!(find_proton(Some(t.path()), None).unwrap(), common.join("Proton Hotfix"));
            let custom = t.path().join("custom");
            assert!(find_proton(Some(t.path()), Some(custom.clone())).is_err());
            touch(&custom.join("proton"));
            assert_eq!(find_proton(Some(t.path()), Some(custom.clone())).unwrap(), custom);
        }
    }
}
