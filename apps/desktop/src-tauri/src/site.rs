//! Rebuilding the website and running its development server on the customer's computer.
//!
//! npm scripts are not run through a shell: `KEY=value program args` is split into environment
//! and arguments, and `program` is resolved to the JavaScript file of the package that provides
//! it, then started with the managed Node.js. This works the same on Windows, macOS and Linux.

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde_json::Value;
use wm_core::runtime::node::NodeRuntime;

use crate::tools;

/// Node.js version used when the package does not name one: the newest LTS release.
pub const DEFAULT_NODE_VERSION: &str = "lts";
/// Characters that need a shell; such scripts are refused.
const SHELL_SYNTAX: &[&str] = &["&&", "||", "|", ";", ">", "<", "$(", "`", "\n"];
/// How long the development server may take to print its address.
const DEV_START_TIMEOUT: Duration = Duration::from_secs(90);
/// Grace period between asking processes to stop and killing them.
const STOP_GRACE: Duration = Duration::from_secs(5);

/// An npm script split into environment, program and arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    pub env: Vec<(String, String)>,
    pub program: String,
    pub args: Vec<String>,
}

/// Splits a script like `ASTRO_TELEMETRY_DISABLED=1 astro build`.
///
/// # Errors
/// Fails for empty scripts and scripts that need a shell (`&&`, pipes, redirects, …).
pub fn parse_script(script: &str) -> Result<Script, String> {
    if let Some(syntax) = SHELL_SYNTAX.iter().find(|syntax| script.contains(**syntax)) {
        return Err(format!("code:script-needs-shell {syntax}"));
    }
    let mut env = Vec::new();
    let mut words = script.split_whitespace().peekable();
    while let Some(word) = words.peek() {
        match word.split_once('=') {
            Some((key, value))
                if !key.is_empty()
                    && key
                        .chars()
                        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_') =>
            {
                env.push((key.to_owned(), value.trim_matches(['"', '\'']).to_owned()));
                words.next();
            }
            _ => break,
        }
    }
    let program = words.next().ok_or("code:script-empty")?.to_owned();
    let args = words
        .map(|word| word.trim_matches(['"', '\'']).to_owned())
        .collect();
    Ok(Script { env, program, args })
}

/// Reads `scripts.<name>` from `package.json`.
///
/// # Errors
/// Fails if `package.json` is missing or has no such script.
pub fn read_script(project: &Path, name: &str) -> Result<Script, String> {
    let text = fs::read_to_string(project.join("package.json")).map_err(|e| e.to_string())?;
    let json: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let script = json["scripts"][name]
        .as_str()
        .ok_or_else(|| format!("code:script-missing {name}"))?;
    parse_script(script)
}

fn bin_of(package_dir: &Path, program: &str) -> Option<PathBuf> {
    let text = fs::read_to_string(package_dir.join("package.json")).ok()?;
    let json: Value = serde_json::from_str(&text).ok()?;
    let relative = match &json["bin"] {
        Value::String(path) if json["name"].as_str()?.rsplit('/').next() == Some(program) => {
            path.clone()
        }
        Value::Object(map) => map.get(program)?.as_str()?.to_owned(),
        _ => return None,
    };
    let path = package_dir.join(relative);
    path.is_file().then_some(path)
}

/// Finds the JavaScript file behind a command of `node_modules/.bin`.
///
/// # Errors
/// Fails if no installed package provides the command.
pub fn resolve_bin(project: &Path, program: &str) -> Result<PathBuf, String> {
    let modules = project.join("node_modules");
    if let Some(path) = bin_of(&modules.join(program), program) {
        return Ok(path);
    }
    for entry in fs::read_dir(&modules)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let candidates: Vec<PathBuf> = if name.starts_with('@') {
            fs::read_dir(&path)
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
                .map(|scoped| scoped.path())
                .collect()
        } else {
            vec![path]
        };
        if let Some(found) = candidates.iter().find_map(|dir| bin_of(dir, program)) {
            return Ok(found);
        }
    }
    Err(format!("code:program-missing {program}"))
}

/// Environment for child processes: managed Node.js and PHP first on `PATH`.
fn command_env(runtime: &NodeRuntime) -> Vec<(String, String)> {
    let mut paths = vec![runtime.bin_dir.clone()];
    let php = tools::php().program;
    if let Some(dir) = php.parent().filter(|dir| php.is_absolute() && dir.is_dir()) {
        paths.push(dir.to_path_buf());
    }
    if let Some(system) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&system));
    }
    let path = std::env::join_paths(paths)
        .map(|joined| joined.to_string_lossy().into_owned())
        .unwrap_or_default();
    vec![
        ("PATH".into(), path),
        ("ASTRO_TELEMETRY_DISABLED".into(), "1".into()),
        // Tools may print colour codes even into a pipe (seen on Windows); the log is plain text.
        ("NO_COLOR".into(), "1".into()),
        ("FORCE_COLOR".into(), "0".into()),
        ("npm_config_update_notifier".into(), "false".into()),
        ("npm_config_fund".into(), "false".into()),
        ("npm_config_audit".into(), "false".into()),
    ]
}

/// Removes ANSI escape sequences (colours, cursor moves) from a line of tool output.
#[must_use]
pub fn strip_ansi(line: &str) -> String {
    let mut plain = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            plain.push(c);
            continue;
        }
        match chars.peek() {
            // CSI: ESC [ parameters … final byte in @..~
            Some('[') => {
                chars.next();
                for code in chars.by_ref() {
                    if ('@'..='~').contains(&code) {
                        break;
                    }
                }
            }
            // OSC: ESC ] … terminated by BEL or ESC \
            Some(']') => {
                chars.next();
                while let Some(code) = chars.next() {
                    if code == '\u{7}' {
                        break;
                    }
                    if code == '\u{1b}' && chars.peek() == Some(&'\\') {
                        chars.next();
                        break;
                    }
                }
            }
            Some(_) => {
                chars.next();
            }
            None => {}
        }
    }
    plain
}

/// Starts processes in their own group so the whole tree can be stopped.
fn own_process_group(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
    }
}

/// Silences a helper command; "no such process" after a clean exit is expected.
fn quiet(command: &mut Command) -> &mut Command {
    command.stdout(Stdio::null()).stderr(Stdio::null())
}

/// Stops a process and everything it started.
pub fn stop_tree(child: &mut Child) {
    let id = child.id().to_string();
    #[cfg(unix)]
    {
        let group = format!("-{id}");
        let _ = quiet(Command::new("kill").args(["-TERM", &group])).status();
        let deadline = Instant::now() + STOP_GRACE;
        while Instant::now() < deadline {
            if matches!(child.try_wait(), Ok(Some(_))) {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        // Grandchildren (e.g. PHP of the CMS) may outlive the parent; end the group for sure.
        let _ = quiet(Command::new("kill").args(["-KILL", &group])).status();
    }
    #[cfg(windows)]
    {
        let _ = quiet(Command::new("taskkill").args(["/PID", &id, "/T", "/F"])).status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// Runs a command to completion and passes every output line to `log`.
fn run_logged(mut command: Command, log: &mut dyn FnMut(&str)) -> Result<(), String> {
    own_process_group(&mut command);
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let (sender, receiver) = mpsc::channel();
    for stream in [
        child
            .stdout
            .take()
            .map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
        child
            .stderr
            .take()
            .map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
    ]
    .into_iter()
    .flatten()
    {
        let sender = sender.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stream).lines().map_while(Result::ok) {
                let _ = sender.send(line);
            }
        });
    }
    drop(sender);
    for line in receiver {
        log(&strip_ansi(&line));
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("code:command-failed {status}"))
    }
}

/// Fails with a clear message if the lockfile still points at private registries the customer
/// cannot reach (package made by an older `pack`, or without `package-lock.json` in `include`).
fn refuse_private_packages(lockfile: &Path) -> Result<(), String> {
    let Ok(text) = fs::read_to_string(lockfile) else {
        return Ok(());
    };
    let Ok(lock) = serde_json::from_str::<Value>(&text) else {
        return Ok(());
    };
    let private = wm_core::runtime::npm::private_packages(&lock);
    if private.is_empty() {
        return Ok(());
    }
    let names: Vec<String> = private
        .iter()
        .map(|package| format!("{}@{}", package.name, package.version))
        .collect();
    Err(format!("code:private-packages {}", names.join(", ")))
}

/// Installs dependencies with `npm ci --ignore-scripts` unless they are already installed
/// for the current lockfile.
///
/// # Errors
/// Fails if npm fails.
pub fn ensure_dependencies(
    project: &Path,
    runtime: &NodeRuntime,
    log: &mut dyn FnMut(&str),
) -> Result<(), String> {
    let installed = project.join("node_modules/.package-lock.json");
    let lockfile = project.join("package-lock.json");
    refuse_private_packages(&lockfile)?;
    let up_to_date = match (fs::metadata(&installed), fs::metadata(&lockfile)) {
        (Ok(installed), Ok(lock)) => installed.modified().ok() >= lock.modified().ok(),
        _ => false,
    };
    if up_to_date {
        log("dependencies are up to date");
        return Ok(());
    }
    log("installing dependencies (npm ci --ignore-scripts) …");
    let mut command = Command::new(&runtime.node);
    command
        .arg(&runtime.npm_cli)
        .args(["ci", "--ignore-scripts", "--no-audit", "--no-fund"])
        .current_dir(project)
        .envs(command_env(runtime));
    run_logged(command, log)
}

fn script_command(
    project: &Path,
    runtime: &NodeRuntime,
    script: &Script,
) -> Result<Command, String> {
    let bin = resolve_bin(project, &script.program)?;
    let mut command = Command::new(&runtime.node);
    command
        .arg(bin)
        .args(&script.args)
        .current_dir(project)
        .envs(command_env(runtime))
        .envs(script.env.iter().cloned());
    Ok(command)
}

/// Runs the `build` script and makes its output the version's `dist/`. The previous `dist/`
/// is kept in `.runtime/dist-before-build/`.
///
/// # Errors
/// Fails if dependencies cannot be installed or the build fails; `dist/` is then unchanged.
pub fn build(
    version_folder: &Path,
    build_output: &str,
    runtime: &NodeRuntime,
    log: &mut dyn FnMut(&str),
) -> Result<(), String> {
    let project = version_folder.join("project");
    ensure_dependencies(&project, runtime, log)?;
    let script = read_script(&project, "build")?;
    log(&format!(
        "building: {} {}",
        script.program,
        script.args.join(" ")
    ));
    run_logged(script_command(&project, runtime, &script)?, log)?;

    let output = project.join(build_output);
    if !output.join("index.html").is_file() {
        return Err(format!("code:build-output-missing {build_output}"));
    }
    let dist = version_folder.join("dist");
    let backup = version_folder.join(".runtime/dist-before-build");
    if backup.exists() {
        fs::remove_dir_all(&backup).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(version_folder.join(".runtime")).map_err(|e| e.to_string())?;
    if dist.exists() {
        fs::rename(&dist, &backup).map_err(|e| e.to_string())?;
    }
    if let Err(error) = fs::rename(&output, &dist) {
        // Put the previous build back so preview and upload keep working.
        let _ = fs::rename(&backup, &dist);
        return Err(error.to_string());
    }
    log("build finished – preview and upload now use the new build");
    Ok(())
}

/// A running development server.
pub struct DevServer {
    child: Child,
    pub url: String,
    pub project: String,
}

impl DevServer {
    /// Stops the server and all processes it started.
    pub fn stop(mut self) {
        stop_tree(&mut self.child);
    }

    /// Whether the server process is still running.
    pub fn is_running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }
}

/// Finds the local address in a line of development server output.
#[must_use]
pub fn local_url(line: &str) -> Option<String> {
    let start = line
        .find("http://localhost")
        .or_else(|| line.find("http://127.0.0.1"))?;
    let url: String = line[start..]
        .chars()
        .take_while(|c| !c.is_whitespace())
        .collect();
    Some(url)
}

/// Starts the `dev` script and waits until it prints its local address. Output keeps going
/// to `log` while the server runs.
///
/// # Errors
/// Fails if dependencies cannot be installed or the server does not start.
pub fn start_dev(
    version_folder: &Path,
    project_name: &str,
    runtime: &NodeRuntime,
    log: impl Fn(&str) + Send + 'static,
) -> Result<DevServer, String> {
    let project = version_folder.join("project");
    ensure_dependencies(&project, runtime, &mut |line| log(line))?;
    let mut script = read_script(&project, "dev")?;
    if script.program == "astro" && script.args.first().is_some_and(|arg| arg == "dev") {
        // Astro ≥ 7.4 may restart itself detached; keep it in our process group.
        script.args.push("--ignore-lock".into());
    }
    log(&format!(
        "starting: {} {}",
        script.program,
        script.args.join(" ")
    ));
    let mut command = script_command(&project, runtime, &script)?;
    own_process_group(&mut command);
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    let (sender, receiver) = mpsc::channel::<String>();
    for stream in [
        child
            .stdout
            .take()
            .map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
        child
            .stderr
            .take()
            .map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
    ]
    .into_iter()
    .flatten()
    {
        let sender = sender.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stream).lines().map_while(Result::ok) {
                let _ = sender.send(line);
            }
        });
    }
    drop(sender);

    let deadline = Instant::now() + DEV_START_TIMEOUT;
    let url = loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        // A closed channel (server ended) or the timeout both mean the start failed.
        let Ok(raw) = receiver.recv_timeout(remaining) else {
            stop_tree(&mut child);
            return Err("code:dev-start-failed".into());
        };
        let line = strip_ansi(&raw);
        log(&line);
        if let Some(url) = local_url(&line) {
            break url;
        }
    };
    std::thread::spawn(move || {
        for line in receiver {
            log(&strip_ansi(&line));
        }
    });
    Ok(DevServer {
        child,
        url,
        project: project_name.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripts_are_split_without_a_shell() {
        let script = parse_script("ASTRO_TELEMETRY_DISABLED=1 astro build --verbose").unwrap();
        assert_eq!(
            script.env,
            vec![("ASTRO_TELEMETRY_DISABLED".into(), "1".into())]
        );
        assert_eq!(script.program, "astro");
        assert_eq!(script.args, vec!["build", "--verbose"]);
        assert_eq!(parse_script("astro dev").unwrap().env, vec![]);
        assert!(parse_script("npm run a && npm run b").is_err());
        assert!(parse_script("astro build > log.txt").is_err());
        assert!(parse_script("A=1").is_err());
    }

    #[test]
    fn bins_are_found_in_plain_and_scoped_packages() {
        let dir = tempfile::tempdir().unwrap();
        let modules = dir.path().join("node_modules");
        fs::create_dir_all(modules.join("astro")).unwrap();
        fs::write(
            modules.join("astro/package.json"),
            r#"{"name":"astro","bin":{"astro":"astro.js"}}"#,
        )
        .unwrap();
        fs::write(modules.join("astro/astro.js"), "").unwrap();
        fs::create_dir_all(modules.join("@scope/tool")).unwrap();
        fs::write(
            modules.join("@scope/tool/package.json"),
            r#"{"name":"@scope/tool","bin":"cli.js"}"#,
        )
        .unwrap();
        fs::write(modules.join("@scope/tool/cli.js"), "").unwrap();

        assert_eq!(
            resolve_bin(dir.path(), "astro").unwrap(),
            modules.join("astro/astro.js")
        );
        assert_eq!(
            resolve_bin(dir.path(), "tool").unwrap(),
            modules.join("@scope/tool/cli.js")
        );
        assert!(resolve_bin(dir.path(), "missing").is_err());
    }

    #[test]
    fn private_registries_are_refused_before_npm_runs() {
        let dir = tempfile::tempdir().unwrap();
        let lockfile = dir.path().join("package-lock.json");
        fs::write(
            &lockfile,
            r#"{"packages":{"node_modules/@x/cms":{"version":"0.3.1","resolved":"https://git.nas.local/npm/@x/cms/-/cms-0.3.1.tgz"}}}"#,
        )
        .unwrap();
        let error = refuse_private_packages(&lockfile).unwrap_err();
        assert_eq!(error, "code:private-packages @x/cms@0.3.1");
        fs::write(
            &lockfile,
            r#"{"packages":{"node_modules/@x/cms":{"version":"0.3.1","resolved":"file:vendor/x-cms-0.3.1.tgz"}}}"#,
        )
        .unwrap();
        assert!(refuse_private_packages(&lockfile).is_ok());
    }

    #[test]
    fn colour_codes_are_removed() {
        let line = "\u{1b}[2m┃\u{1b}[22m Local    \u{1b}[36mhttp://localhost:4321/\u{1b}[39m";
        assert_eq!(strip_ansi(line), "┃ Local    http://localhost:4321/");
        assert_eq!(
            local_url(&strip_ansi(line)).as_deref(),
            Some("http://localhost:4321/")
        );
        assert_eq!(strip_ansi("\u{1b}]0;title\u{7}plain"), "plain");
        assert_eq!(strip_ansi("ohne Farben"), "ohne Farben");
    }

    #[test]
    fn local_addresses_are_found() {
        assert_eq!(
            local_url("  ┃ Local    http://localhost:4321/").as_deref(),
            Some("http://localhost:4321/")
        );
        assert_eq!(
            local_url("ready at http://127.0.0.1:8080 now").as_deref(),
            Some("http://127.0.0.1:8080")
        );
        assert!(local_url("Network  use --host to expose").is_none());
    }
}
