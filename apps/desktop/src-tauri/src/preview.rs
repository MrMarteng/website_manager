//! Local preview of the built website with the PHP built-in server.
//!
//! The server listens on 127.0.0.1 only (no firewall prompt, not reachable from the
//! network), may only touch the package folder (`open_basedir`) and cannot start
//! processes (`disable_functions`). A router applies the `.htaccess` access rules that the
//! built-in server ignores (see `router.php`).

use std::io::{BufRead, BufReader};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::push_log;
use crate::tools;

/// PHP functions that could start other programs.
const DISABLED_FUNCTIONS: &str = "exec,shell_exec,system,passthru,proc_open,popen,pcntl_exec";

/// Router script written by the app (never taken from the package).
const ROUTER: &str = include_str!("router.php");

/// A running preview server.
pub struct Preview {
    child: Child,
    pub url: String,
    pub folder: String,
}

impl Preview {
    /// Stops the server.
    pub fn stop(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    /// Whether the server process is still running.
    pub fn is_running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }
}

/// Starts PHP for `<folder>/dist` on a free local port and waits until it answers.
///
/// # Errors
/// Fails if PHP cannot be started or does not answer within a few seconds.
pub fn start(folder: &Path, log: &Arc<Mutex<Vec<String>>>) -> Result<Preview, String> {
    let dist = folder.join("dist");
    if !dist.is_dir() {
        return Err("package contains no built website (dist)".into());
    }
    let runtime = folder.join(".runtime");
    let sessions = runtime.join("sessions");
    let temp = runtime.join("tmp");
    for dir in [&sessions, &temp] {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let router = runtime.join("router.php");
    std::fs::write(&router, ROUTER).map_err(|e| e.to_string())?;

    let port = free_port()?;
    let address = format!("127.0.0.1:{port}");
    let separator = if cfg!(windows) { ";" } else { ":" };
    let php = tools::php();
    push_log(
        log,
        format!("PHP ({}): {}", php.source.label(), php.program.display()),
    );

    let mut command = Command::new(&php.program);
    if php.source == tools::Source::Bundled {
        // The bundled PHP has its extensions built in; ignore any php.ini on the computer.
        command.arg("-n");
    }
    let mut child = command
        .arg("-S")
        .arg(&address)
        .arg("-t")
        .arg(&dist)
        .arg("-d")
        .arg(format!("open_basedir={}{separator}", folder.display()))
        .arg("-d")
        .arg(format!("disable_functions={DISABLED_FUNCTIONS}"))
        .arg("-d")
        .arg(format!("session.save_path={}", sessions.display()))
        .arg("-d")
        .arg(format!("upload_tmp_dir={}", temp.display()))
        .arg("-d")
        .arg(format!("sys_temp_dir={}", temp.display()))
        .arg(&router)
        .current_dir(folder)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("PHP could not be started ({}): {e}", php.program.display()))?;

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
        let log = Arc::clone(log);
        std::thread::spawn(move || {
            for line in BufReader::new(stream).lines().map_while(Result::ok) {
                push_log(&log, format!("php: {line}"));
            }
        });
    }

    let socket: SocketAddr = address
        .parse()
        .map_err(|e: std::net::AddrParseError| e.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(5);
    while TcpStream::connect_timeout(&socket, Duration::from_millis(200)).is_err() {
        if Instant::now() > deadline || !matches!(child.try_wait(), Ok(None)) {
            let _ = child.kill();
            return Err("PHP preview did not start – see log".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    Ok(Preview {
        child,
        url: format!("http://{address}/"),
        folder: folder.to_string_lossy().into_owned(),
    })
}

fn free_port() -> Result<u16, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    listener
        .local_addr()
        .map(|address| address.port())
        .map_err(|e| e.to_string())
}
