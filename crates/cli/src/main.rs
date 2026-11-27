//! `website-manager`: create, verify and upload Website Manager packages.

mod pack;
mod time;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use teleschirm_deploy::{Connection, Credential, DeployConfig, MappingMode, Runner};
use wm_core::signing;
use wm_core::trust::{TrustDecision, TrustStore};

#[derive(Parser)]
#[command(
    name = "website-manager",
    version,
    about = "Create, verify and upload website packages"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a signing key pair for publishing packages.
    Keygen {
        /// Folder for `signing.key` (secret) and `signing.pub`.
        #[arg(long)]
        out: PathBuf,
    },
    /// Build a project and pack it into a signed `.tsweb` package.
    Pack {
        /// Website project with a `website-manager.config.json`.
        #[arg(long, default_value = ".")]
        project: PathBuf,
        /// Secret signing key created with `keygen`.
        #[arg(long)]
        key: PathBuf,
        /// Output file; default `<name>-<version>.tsweb` in the current folder.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Version of this delivery; overrides the config.
        #[arg(long = "release")]
        release: Option<String>,
        /// Use the existing build output instead of building.
        #[arg(long)]
        skip_build: bool,
    },
    /// Verify a package as the desktop app does when opening it.
    Verify {
        package: PathBuf,
        /// Trust store; binds the publisher on first use and rejects other keys later.
        #[arg(long)]
        trust_store: Option<PathBuf>,
    },
    /// Upload according to a `deploy.conf`.
    Deploy {
        #[arg(long, default_value = "deploy.conf")]
        config: PathBuf,
        /// Show what would change without changing anything.
        #[arg(long, short = 'n')]
        dry_run: bool,
        #[arg(long, value_enum, default_value_t = RunMode::Sync)]
        mode: RunMode,
        /// Upload into a local folder instead of the server (for tests).
        #[arg(long)]
        local_folder: Option<PathBuf>,
        /// Name of an environment variable holding the server password (instead of a key).
        #[arg(long)]
        password_env: Option<String>,
        /// rclone program.
        #[arg(long, default_value = "rclone")]
        rclone: PathBuf,
    },
    /// Download and verify a Node.js version into a cache folder.
    Node {
        version: String,
        #[arg(long)]
        cache: PathBuf,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum RunMode {
    Sync,
    Copy,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("Fehler: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Commands::Keygen { out } => keygen(&out),
        Commands::Pack {
            project,
            key,
            out,
            release,
            skip_build,
        } => {
            let options = pack::PackOptions {
                project,
                key,
                output: out,
                release,
                skip_build,
            };
            let report = pack::pack(&options).map_err(|e| e.to_string())?;
            println!(
                "Paket erstellt: {} ({} Dateien, Version {})",
                report.output.display(),
                report.files,
                report.version
            );
            for warning in report.warnings {
                println!("Hinweis: {warning}");
            }
            Ok(())
        }
        Commands::Verify {
            package,
            trust_store,
        } => verify(&package, trust_store.as_deref()),
        Commands::Deploy {
            config,
            dry_run,
            mode,
            local_folder,
            password_env,
            rclone,
        } => deploy(
            &config,
            dry_run,
            mode,
            local_folder,
            password_env.as_deref(),
            rclone,
        ),
        Commands::Node { version, cache } => {
            let runtime =
                wm_core::runtime::node::ensure(&version, &cache).map_err(|e| e.to_string())?;
            println!("{}", runtime.node.display());
            Ok(())
        }
    }
}

fn keygen(out: &std::path::Path) -> Result<(), String> {
    let secret = out.join("signing.key");
    let public = out.join("signing.pub");
    if secret.exists() || public.exists() {
        return Err(format!(
            "{} enthält bereits Schlüssel – nichts überschrieben",
            out.display()
        ));
    }
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let key = signing::generate_signing_key().map_err(|e| e.to_string())?;
    signing::save_signing_key(&key, &secret).map_err(|e| e.to_string())?;
    std::fs::write(&public, signing::encode_public_key(&key.verifying_key()))
        .map_err(|e| e.to_string())?;
    println!(
        "Schlüssel erstellt, Fingerabdruck {}\n  geheim:     {}\n  öffentlich: {}",
        signing::fingerprint(&key.verifying_key()),
        secret.display(),
        public.display()
    );
    Ok(())
}

fn verify(package: &std::path::Path, trust_store: Option<&std::path::Path>) -> Result<(), String> {
    let verified = wm_core::archive::verify_package(package).map_err(|e| e.to_string())?;
    let manifest = &verified.manifest;
    println!(
        "Paket in Ordnung: {} {} vom {}\n  Absender:  {}\n  Dateien:   {}",
        manifest.name,
        manifest.version,
        manifest.created,
        signing::fingerprint(&verified.publisher),
        manifest.files.len()
    );
    if let Some(path) = trust_store {
        let mut store = TrustStore::load(path).map_err(|e| e.to_string())?;
        match store
            .check(&manifest.name, &verified.publisher)
            .map_err(|e| e.to_string())?
        {
            TrustDecision::Known => println!("  Absender bekannt."),
            TrustDecision::FirstUse => {
                store
                    .bind(&manifest.name, &verified.publisher)
                    .map_err(|e| e.to_string())?;
                println!("  Absender neu – für dieses Projekt gespeichert.");
            }
        }
    }
    Ok(())
}

fn deploy(
    config_path: &std::path::Path,
    dry_run: bool,
    mode: RunMode,
    local_folder: Option<PathBuf>,
    password_env: Option<&str>,
    rclone: PathBuf,
) -> Result<(), String> {
    let config = DeployConfig::load(config_path).map_err(|e| e.to_string())?;
    let connection = match (local_folder, password_env) {
        (Some(folder), _) => Connection::LocalFolder(folder),
        (None, Some(variable)) => Connection::Server(Credential::Password(
            std::env::var(variable).map_err(|_| format!("Umgebungsvariable {variable} fehlt"))?,
        )),
        (None, None) => Connection::Server(Credential::KeyFile),
    };
    let run_mode = match mode {
        RunMode::Sync => MappingMode::Sync,
        RunMode::Copy => MappingMode::Copy,
    };
    let summary = Runner::new(rclone, dry_run)
        .run(&config, run_mode, &connection, None, &mut |line| {
            println!("{line}");
        })
        .map_err(|e| e.to_string())?;
    println!(
        "=== {}/{} Zuordnung(en) OK{} ===",
        summary.done,
        summary.total,
        if summary.dry_run {
            " (Trockenlauf – nichts geändert)"
        } else {
            ""
        }
    );
    Ok(())
}
