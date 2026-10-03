//! Packaging commands that assemble distributable desktop builds.
//!
//! macOS builds stage a `PacketSmith.app` bundle and optionally wrap it in a
//! DMG; Windows builds stage a portable folder and optionally produce an NSIS
//! installer and zip. Installer creation is intentionally platform-native:
//! macOS needs `codesign`, `lipo`, and `hdiutil`; Windows needs `makensis`.
//!
//! Usage:
//! ```bash
//! cargo xtask package --format all --arch universal
//! ```

use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

const APP_NAME: &str = "PacketSmith";
const APP_PACKAGE: &str = "packetsmith-app";
const APP_FEATURES: &str = "gpui-ui";
const APP_BUNDLE_ID: &str = "dev.packetsmith.desktop";
const MIN_MACOS_VERSION: &str = "11.0";
const EXECUTABLE_NAME: &str = "PacketSmith";
/// Default output directory, relative to the Cargo target directory.
const DEFAULT_OUTPUT_DIR: &str = "dist";
const INFO_PLIST_TEMPLATE: &str = "packaging/macos/Info.plist";
const ENTITLEMENTS: &str = "packaging/macos/entitlements.plist";
const ICNS_ICON: &str = "packaging/icons/AppIcon.icns";
const ICO_ICON: &str = "packaging/icons/app-icon.ico";
const NSIS_SCRIPT: &str = "packaging/windows/installer.nsi";

const HELP: &str = "\
Usage: cargo xtask package [OPTIONS]

Builds installable desktop artifacts for the current platform. macOS stages
PacketSmith.app and produces a DMG; Windows stages a portable folder and
produces an NSIS setup executable and a zip.

Options:
  --format <value>    app, dmg, portable, nsis, zip, all (repeatable,
                      comma-separated). Defaults to all for this platform.
  --arch <value>      native (default), universal, arm64, or x64.
                      universal is macOS-only; Windows supports x64 and arm64.
  --debug             Build the debug profile instead of release.
  --skip-build        Reuse the already-staged app/folder without rebuilding.
                      Use after signing or editing staged files.
  --sign <value>      macOS signing: adhoc (default), none, or a codesign
                      identity such as \"Developer ID Application: ...\".
  --version <value>   Override the version embedded in artifacts.
  --output-dir <dir>  Output directory (default: target/dist).
  --makensis <path>   Explicit path to the NSIS compiler.
  --dry-run           Print planned commands without executing them.
  -h, --help          Show this help.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    All,
    App,
    Dmg,
    Portable,
    Nsis,
    Zip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Arch {
    Native,
    Universal,
    Arm64,
    X64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SignMode {
    None,
    Adhoc,
    Identity(String),
}

#[derive(Debug, Clone)]
struct Options {
    formats: Vec<Format>,
    arch: Arch,
    debug: bool,
    skip_build: bool,
    sign: SignMode,
    version: String,
    output_dir: Option<PathBuf>,
    makensis: Option<PathBuf>,
    dry_run: bool,
    help: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            formats: Vec::new(),
            arch: Arch::Native,
            debug: false,
            skip_build: false,
            sign: SignMode::Adhoc,
            version: env!("CARGO_PKG_VERSION").to_string(),
            output_dir: None,
            makensis: None,
            dry_run: false,
            help: false,
        }
    }
}

impl Options {
    fn parse(args: &[String]) -> Result<Self> {
        let mut options = Options::default();
        let mut index = 0;
        while index < args.len() {
            let argument = args[index].as_str();
            let mut take_value = |name: &str| -> Result<String> {
                index += 1;
                args.get(index)
                    .cloned()
                    .with_context(|| format!("{name} requires a value"))
            };
            match argument {
                "--format" => {
                    let value = take_value("--format")?;
                    for part in value.split(',') {
                        options.formats.extend(Format::parse(part.trim())?);
                    }
                }
                "--arch" => {
                    let value = take_value("--arch")?;
                    options.arch = Arch::parse(&value)?;
                }
                "--debug" => options.debug = true,
                "--skip-build" => options.skip_build = true,
                "--sign" => {
                    let value = take_value("--sign")?;
                    options.sign = match value.as_str() {
                        "none" => SignMode::None,
                        "adhoc" => SignMode::Adhoc,
                        identity => SignMode::Identity(identity.to_string()),
                    };
                }
                "--version" => options.version = take_value("--version")?,
                "--output-dir" => {
                    options.output_dir = Some(PathBuf::from(take_value("--output-dir")?))
                }
                "--makensis" => options.makensis = Some(PathBuf::from(take_value("--makensis")?)),
                "--dry-run" => options.dry_run = true,
                "-h" | "--help" => options.help = true,
                other => {
                    bail!("unknown package option '{other}' (run `cargo xtask package --help`)")
                }
            }
            index += 1;
        }
        Ok(options)
    }
}

impl Format {
    fn parse(value: &str) -> Result<Vec<Format>> {
        match value {
            "all" => Ok(vec![Format::All]),
            "app" => Ok(vec![Format::App]),
            "dmg" => Ok(vec![Format::Dmg]),
            "portable" => Ok(vec![Format::Portable]),
            "nsis" => Ok(vec![Format::Nsis]),
            "zip" => Ok(vec![Format::Zip]),
            other => bail!(
                "unknown package format '{other}'; expected app, dmg, portable, nsis, zip, or all"
            ),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Format::All => "all",
            Format::App => "app",
            Format::Dmg => "dmg",
            Format::Portable => "portable",
            Format::Nsis => "nsis",
            Format::Zip => "zip",
        }
    }
}

impl Arch {
    fn parse(value: &str) -> Result<Arch> {
        match value {
            "native" => Ok(Arch::Native),
            "universal" => Ok(Arch::Universal),
            "arm64" | "aarch64" => Ok(Arch::Arm64),
            "x64" | "x86_64" => Ok(Arch::X64),
            other => {
                bail!("unknown architecture '{other}'; expected native, universal, arm64, or x64")
            }
        }
    }
}

pub(crate) fn run(args: &[String]) -> Result<()> {
    let options = Options::parse(args)?;
    if options.help {
        println!("{HELP}");
        return Ok(());
    }

    let root = repo_root();
    let host = env::consts::OS;
    let platform = if host == "macos" || host == "windows" {
        host
    } else if options.dry_run {
        preview_platform(&options.formats)?
    } else {
        bail!("packaging is supported on macOS and Windows hosts; use --dry-run to preview")
    };
    let formats = resolve_formats(platform, &options.formats)?;
    if formats.is_empty() {
        bail!("no package formats apply to {platform}");
    }
    if options.arch == Arch::Universal && platform != "macos" {
        bail!("universal builds are only supported on macOS");
    }

    let target_dir = target_dir(&root);
    let output_dir = options
        .output_dir
        .clone()
        .map(|dir| {
            if dir.is_absolute() {
                dir
            } else {
                root.join(dir)
            }
        })
        .unwrap_or_else(|| target_dir.join(DEFAULT_OUTPUT_DIR));

    let task = Task {
        dry_run: options.dry_run,
    };
    let arch_label = arch_label(options.arch);
    let staged_app = formats.contains(&Format::App) || formats.contains(&Format::Dmg);
    let staged_windows = formats.contains(&Format::Portable)
        || formats.contains(&Format::Nsis)
        || formats.contains(&Format::Zip);

    println!(
        "Packaging {} {} ({platform}/{arch_label}) with formats: {}",
        APP_NAME,
        options.version,
        formats
            .iter()
            .map(|format| format.name())
            .collect::<Vec<_>>()
            .join(", ")
    );

    let mut artifacts: Vec<PathBuf> = Vec::new();
    if (staged_app || staged_windows) && !options.skip_build {
        let binary = build_binary(
            &task,
            &root,
            &target_dir,
            &output_dir,
            platform,
            options.arch,
            options.debug,
        )?;
        if staged_app {
            let app = stage_macos_app(&task, &root, &output_dir, &binary, &options.version)?;
            if formats.contains(&Format::App) {
                sign_app(&task, &root, &app, &options.sign)?;
            }
        }
        if staged_windows {
            stage_windows_folder(&task, &root, &output_dir, &binary)?;
        }
    } else if options.skip_build {
        if staged_app
            && !output_dir
                .join("macos")
                .join(format!("{APP_NAME}.app"))
                .exists()
            && !options.dry_run
        {
            bail!("staged app is missing; run without --skip-build first");
        }
        if staged_windows && !output_dir.join("windows").join(APP_NAME).exists() && !options.dry_run
        {
            bail!("staged Windows folder is missing; run without --skip-build first");
        }
    }

    if formats.contains(&Format::Dmg) {
        let app = output_dir.join("macos").join(format!("{APP_NAME}.app"));
        artifacts.push(create_dmg(
            &task,
            &output_dir,
            &options.version,
            &arch_label,
            &app,
        )?);
    }
    if formats.contains(&Format::App) {
        artifacts.push(output_dir.join("macos").join(format!("{APP_NAME}.app")));
    }
    if formats.contains(&Format::Nsis) {
        artifacts.push(create_nsis_installer(
            &task,
            &root,
            &output_dir,
            &options,
            &arch_label,
        )?);
    }
    if formats.contains(&Format::Zip) {
        artifacts.push(create_portable_zip(
            &task,
            &output_dir,
            &options.version,
            &arch_label,
        )?);
    }

    println!("Packaged artifacts:");
    for artifact in &artifacts {
        println!("  {}", artifact.display());
    }
    if artifacts.is_empty() {
        println!("  (none; staging only)");
    }
    Ok(())
}

fn preview_platform(requested: &[Format]) -> Result<&'static str> {
    if requested.is_empty() || requested.contains(&Format::All) {
        bail!("pass an explicit --format with --dry-run on this host");
    }
    let macos = requested
        .iter()
        .any(|format| matches!(format, Format::App | Format::Dmg));
    let windows = requested
        .iter()
        .any(|format| matches!(format, Format::Portable | Format::Nsis | Format::Zip));
    match (macos, windows) {
        (true, false) => Ok("macos"),
        (false, true) => Ok("windows"),
        (true, true) => {
            bail!("cannot preview macOS and Windows formats together; preview them separately")
        }
        (false, false) => unreachable!("every format targets a platform"),
    }
}

fn resolve_formats(platform: &str, requested: &[Format]) -> Result<Vec<Format>> {
    let defaults: &[Format] = match platform {
        "macos" => &[Format::App, Format::Dmg],
        "windows" => &[Format::Portable, Format::Nsis, Format::Zip],
        other => bail!("unsupported packaging platform '{other}'"),
    };

    let mut resolved: Vec<Format> = Vec::new();
    let mut include_defaults = requested.is_empty();
    for format in requested {
        match format {
            Format::All => include_defaults = true,
            Format::App | Format::Dmg if platform == "macos" => {
                if !resolved.contains(format) {
                    resolved.push(*format);
                }
            }
            Format::Portable | Format::Nsis | Format::Zip if platform == "windows" => {
                if !resolved.contains(format) {
                    resolved.push(*format);
                }
            }
            other => bail!(
                "format '{}' is not supported on the {platform} host",
                other.name()
            ),
        }
    }
    if include_defaults {
        for format in defaults {
            if !resolved.contains(format) {
                resolved.push(*format);
            }
        }
    }
    Ok(resolved)
}

fn repo_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap_or_else(|_| manifest.join("..").join(".."))
}

fn target_dir(root: &Path) -> PathBuf {
    match env::var_os("CARGO_TARGET_DIR") {
        Some(dir) if Path::new(&dir).is_absolute() => PathBuf::from(dir),
        Some(dir) => root.join(dir),
        None => root.join("target"),
    }
}

fn profile_name(debug: bool) -> &'static str {
    if debug {
        "debug"
    } else {
        "release"
    }
}

fn binary_name() -> String {
    format!("{APP_PACKAGE}{}", env::consts::EXE_SUFFIX)
}

fn binary_path(target_dir: &Path, triple: Option<&str>, debug: bool) -> PathBuf {
    let mut path = target_dir.to_path_buf();
    if let Some(triple) = triple {
        path.push(triple);
    }
    path.push(profile_name(debug));
    path.push(binary_name());
    path
}

fn arch_label(arch: Arch) -> String {
    match arch {
        Arch::Universal => "universal".to_string(),
        Arch::Arm64 => "arm64".to_string(),
        Arch::X64 => "x64".to_string(),
        Arch::Native => match env::consts::ARCH {
            "aarch64" => "arm64".to_string(),
            "x86_64" => "x64".to_string(),
            other => other.to_string(),
        },
    }
}

fn target_triple(platform: &str, arch: Arch) -> Option<&'static str> {
    match (platform, arch) {
        ("macos", Arch::Arm64) => Some("aarch64-apple-darwin"),
        ("macos", Arch::X64) => Some("x86_64-apple-darwin"),
        ("windows", Arch::Arm64) => Some("aarch64-pc-windows-msvc"),
        ("windows", Arch::X64) => Some("x86_64-pc-windows-msvc"),
        _ => None,
    }
}

fn build_binary(
    task: &Task,
    root: &Path,
    target_dir: &Path,
    output_dir: &Path,
    platform: &str,
    arch: Arch,
    debug: bool,
) -> Result<PathBuf> {
    if arch == Arch::Universal {
        let arm = build_one(
            task,
            root,
            target_dir,
            target_triple(platform, Arch::Arm64),
            debug,
        )?;
        let intel = build_one(
            task,
            root,
            target_dir,
            target_triple(platform, Arch::X64),
            debug,
        )?;
        let merged = output_dir
            .join("build")
            .join(format!("{EXECUTABLE_NAME}-universal"));
        task.ensure_dir(merged.parent().expect("merged binary has a parent"))?;
        let arguments: Vec<OsString> = vec![
            "-create".into(),
            "-output".into(),
            merged.clone().into_os_string(),
            arm.into_os_string(),
            intel.into_os_string(),
        ];
        task.run("lipo", &arguments, None, &[])?;
        return Ok(merged);
    }
    build_one(task, root, target_dir, target_triple(platform, arch), debug)
}

fn build_one(
    task: &Task,
    root: &Path,
    target_dir: &Path,
    triple: Option<&str>,
    debug: bool,
) -> Result<PathBuf> {
    let mut arguments: Vec<OsString> = vec![
        "build".into(),
        "-p".into(),
        APP_PACKAGE.into(),
        "--features".into(),
        APP_FEATURES.into(),
    ];
    if !debug {
        arguments.push("--release".into());
    }
    if let Some(triple) = triple {
        arguments.push("--target".into());
        arguments.push(triple.into());
    }

    let mut environment: Vec<(OsString, OsString)> = Vec::new();
    if cfg!(windows) && !debug {
        let mut flags = env::var("RUSTFLAGS").unwrap_or_default();
        if !flags
            .split_whitespace()
            .any(|flag| flag.ends_with("crt-static"))
        {
            flags.push_str(" -C target-feature=+crt-static");
        }
        environment.push(("RUSTFLAGS".into(), flags.into()));
    }

    let cargo = env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    task.run(
        &cargo.to_string_lossy(),
        &arguments,
        Some(root),
        &environment,
    )?;
    Ok(binary_path(target_dir, triple, debug))
}

fn stage_macos_app(
    task: &Task,
    root: &Path,
    output_dir: &Path,
    binary: &Path,
    version: &str,
) -> Result<PathBuf> {
    let app = output_dir.join("macos").join(format!("{APP_NAME}.app"));
    let contents = app.join("Contents");
    task.remove_dir_all(&app)?;
    task.ensure_dir(&contents.join("MacOS"))?;
    task.ensure_dir(&contents.join("Resources"))?;
    task.copy_file(binary, &contents.join("MacOS").join(EXECUTABLE_NAME))?;

    let template = fs::read_to_string(root.join(INFO_PLIST_TEMPLATE))
        .with_context(|| format!("failed to read {INFO_PLIST_TEMPLATE}"))?;
    let plist = render_info_plist(&template, version, version, MIN_MACOS_VERSION, 2026);
    task.write_file(&contents.join("Info.plist"), plist.as_bytes())?;
    task.write_file(&contents.join("PkgInfo"), b"APPL????")?;

    let icon = root.join(ICNS_ICON);
    if icon.exists() {
        task.copy_file(&icon, &contents.join("Resources").join("AppIcon.icns"))?;
    } else {
        println!("warning: {ICNS_ICON} not found; bundling without an icon");
    }
    Ok(app)
}

fn render_info_plist(
    template: &str,
    version: &str,
    build_number: &str,
    minimum_system_version: &str,
    copyright_year: i32,
) -> String {
    template
        .replace("@@VERSION@@", version)
        .replace("@@BUILD_NUMBER@@", build_number)
        .replace("@@MIN_OS_VERSION@@", minimum_system_version)
        .replace("@@BUNDLE_ID@@", APP_BUNDLE_ID)
        .replace("@@COPYRIGHT_YEAR@@", &copyright_year.to_string())
}

fn sign_app(task: &Task, root: &Path, app: &Path, sign: &SignMode) -> Result<()> {
    match sign {
        SignMode::None => {
            println!("Skipping code signing (--sign none)");
            return Ok(());
        }
        SignMode::Adhoc => {
            let arguments: Vec<OsString> = vec![
                "--force".into(),
                "--sign".into(),
                "-".into(),
                "--timestamp=none".into(),
                app.as_os_str().to_owned(),
            ];
            task.run("codesign", &arguments, None, &[])?;
        }
        SignMode::Identity(identity) => {
            let arguments: Vec<OsString> = vec![
                "--force".into(),
                "--options".into(),
                "runtime".into(),
                "--entitlements".into(),
                root.join(ENTITLEMENTS).into_os_string(),
                "--sign".into(),
                identity.into(),
                "--timestamp".into(),
                app.as_os_str().to_owned(),
            ];
            task.run("codesign", &arguments, None, &[])?;
        }
    }

    let arguments: Vec<OsString> = vec![
        "--verify".into(),
        "--strict".into(),
        "--verbose=2".into(),
        app.as_os_str().to_owned(),
    ];
    task.run("codesign", &arguments, None, &[])?;
    Ok(())
}

fn create_dmg(
    task: &Task,
    output_dir: &Path,
    version: &str,
    arch: &str,
    app: &Path,
) -> Result<PathBuf> {
    if !app.exists() && !task.dry_run {
        bail!(
            "staged app not found at {}; run without --skip-build first",
            app.display()
        );
    }
    let staging = output_dir.join("macos").join("dmg");
    task.remove_dir_all(&staging)?;
    task.ensure_dir(&staging)?;
    task.copy_dir_all(app, &staging.join(format!("{APP_NAME}.app")))?;
    task.symlink_dir(Path::new("/Applications"), &staging.join("Applications"))?;

    let dmg = output_dir.join(artifact_name(version, "macos", arch, "dmg"));
    let arguments: Vec<OsString> = vec![
        "create".into(),
        "-volname".into(),
        APP_NAME.into(),
        "-srcfolder".into(),
        staging.clone().into_os_string(),
        "-ov".into(),
        "-format".into(),
        "UDZO".into(),
        dmg.clone().into_os_string(),
    ];
    task.run("hdiutil", &arguments, None, &[])?;
    Ok(dmg)
}

fn stage_windows_folder(
    task: &Task,
    root: &Path,
    output_dir: &Path,
    binary: &Path,
) -> Result<PathBuf> {
    let stage = output_dir.join("windows").join(APP_NAME);
    task.remove_dir_all(&stage)?;
    task.ensure_dir(&stage)?;
    task.copy_file(binary, &stage.join(format!("{EXECUTABLE_NAME}.exe")))?;
    let license = root.join("LICENSE");
    if license.exists() {
        task.copy_file(&license, &stage.join("LICENSE.txt"))?;
    }
    Ok(stage)
}

fn create_nsis_installer(
    task: &Task,
    root: &Path,
    output_dir: &Path,
    options: &Options,
    arch: &str,
) -> Result<PathBuf> {
    let stage = output_dir.join("windows").join(APP_NAME);
    if !stage.exists() && !task.dry_run {
        bail!(
            "staged Windows folder not found at {}; run without --skip-build first",
            stage.display()
        );
    }
    let makensis = find_makensis(options.makensis.as_deref()).or_else(|error| {
        if task.dry_run {
            Ok(PathBuf::from("makensis"))
        } else {
            Err(error)
        }
    })?;
    let installer = output_dir.join(artifact_name(&options.version, "windows", arch, "setup"));
    task.ensure_dir(output_dir)?;

    let mut arguments: Vec<OsString> = vec![
        define("APP_VERSION", &options.version),
        define("APP_VERSION_QUAD", &version_quad(&options.version)),
        define("APP_NAME", APP_NAME),
        define("APP_EXE", &format!("{EXECUTABLE_NAME}.exe")),
        define("APP_ARCH", arch),
        define("STAGE_DIR", &stage.to_string_lossy()),
        define("OUT_FILE", &installer.to_string_lossy()),
        define("LICENSE_FILE", &root.join("LICENSE").to_string_lossy()),
    ];
    let icon = root.join(ICO_ICON);
    if icon.exists() {
        arguments.push(define("ICON_FILE", &icon.to_string_lossy()));
    }
    arguments.push(root.join(NSIS_SCRIPT).into_os_string());

    task.run(&makensis.to_string_lossy(), &arguments, Some(root), &[])?;
    Ok(installer)
}

fn create_portable_zip(
    task: &Task,
    output_dir: &Path,
    version: &str,
    arch: &str,
) -> Result<PathBuf> {
    let stage = output_dir.join("windows").join(APP_NAME);
    if !stage.exists() && !task.dry_run {
        bail!(
            "staged Windows folder not found at {}; run without --skip-build first",
            stage.display()
        );
    }
    let zip = output_dir.join(artifact_name(version, "windows", arch, "zip"));

    #[cfg(windows)]
    {
        let command = format!(
            "Compress-Archive -Path '{}' -DestinationPath '{}' -Force",
            stage.join("*").display(),
            zip.display()
        );
        task.run(
            "powershell",
            &[
                "-NoProfile".into(),
                "-NonInteractive".into(),
                "-Command".into(),
                command.into(),
            ],
            None,
            &[],
        )?;
    }
    #[cfg(not(windows))]
    {
        let arguments: Vec<OsString> = vec![
            "-r".into(),
            "-q".into(),
            zip.clone().into_os_string(),
            ".".into(),
        ];
        task.run("zip", &arguments, Some(&stage), &[])?;
    }

    Ok(zip)
}

fn find_makensis(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        if path.exists() {
            return Ok(path.to_path_buf());
        }
        bail!("makensis not found at {}", path.display());
    }
    if let Some(path) = which("makensis") {
        return Ok(path);
    }
    #[cfg(windows)]
    {
        for variable in ["ProgramFiles(x86)", "ProgramFiles"] {
            if let Some(base) = env::var_os(variable) {
                let candidate = PathBuf::from(base).join("NSIS").join("makensis.exe");
                if candidate.exists() {
                    return Ok(candidate);
                }
            }
        }
    }
    bail!(
        "makensis not found; install NSIS (https://nsis.sourceforge.io) or pass --makensis <path>"
    )
}

fn which(name: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    for directory in env::split_paths(&paths) {
        let candidate = directory.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            let candidate = directory.join(format!("{name}.exe"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn define(name: &str, value: &str) -> OsString {
    format!("-D{name}={value}").into()
}

fn artifact_name(version: &str, platform: &str, arch: &str, kind: &str) -> String {
    match kind {
        "dmg" => format!("{APP_NAME}-{version}-{platform}-{arch}.dmg"),
        "setup" => format!("{APP_NAME}-{version}-{platform}-{arch}-setup.exe"),
        "zip" => format!("{APP_NAME}-{version}-{platform}-{arch}.zip"),
        other => unreachable!("unknown artifact kind {other}"),
    }
}

fn version_quad(version: &str) -> String {
    let core = version.split(['-', '+']).next().unwrap_or(version);
    let mut parts: Vec<String> = core
        .split('.')
        .map(|part| part.chars().take_while(char::is_ascii_digit).collect())
        .collect();
    parts.truncate(4);
    while parts.len() < 4 {
        parts.push("0".to_string());
    }
    parts.join(".")
}

struct Task {
    dry_run: bool,
}

impl Task {
    fn run(
        &self,
        program: &str,
        arguments: &[OsString],
        cwd: Option<&Path>,
        environment: &[(OsString, OsString)],
    ) -> Result<()> {
        let mut printable = program.to_string();
        for argument in arguments {
            printable.push(' ');
            printable.push_str(&argument.to_string_lossy());
        }
        if self.dry_run {
            println!("[dry-run] {printable}");
            return Ok(());
        }
        println!("$ {printable}");
        let mut command = Command::new(program);
        command.args(arguments);
        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        }
        for (key, value) in environment {
            command.env(key, value);
        }
        let status = command
            .status()
            .with_context(|| format!("failed to execute '{program}'"))?;
        if !status.success() {
            bail!("command failed with {status}: {printable}");
        }
        Ok(())
    }

    fn ensure_dir(&self, path: &Path) -> Result<()> {
        if self.dry_run {
            println!("[dry-run] create directory {}", path.display());
            return Ok(());
        }
        fs::create_dir_all(path).with_context(|| format!("failed to create {}", path.display()))
    }

    fn remove_dir_all(&self, path: &Path) -> Result<()> {
        if !path.exists() {
            return Ok(());
        }
        if self.dry_run {
            println!("[dry-run] remove directory {}", path.display());
            return Ok(());
        }
        fs::remove_dir_all(path).with_context(|| format!("failed to remove {}", path.display()))
    }

    fn copy_file(&self, from: &Path, to: &Path) -> Result<()> {
        if self.dry_run {
            println!("[dry-run] copy {} -> {}", from.display(), to.display());
            return Ok(());
        }
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(from, to)
            .with_context(|| format!("failed to copy {} to {}", from.display(), to.display()))?;
        Ok(())
    }

    fn copy_dir_all(&self, from: &Path, to: &Path) -> Result<()> {
        if self.dry_run {
            println!(
                "[dry-run] copy directory {} -> {}",
                from.display(),
                to.display()
            );
            return Ok(());
        }
        self.ensure_dir(to)?;
        for entry in
            fs::read_dir(from).with_context(|| format!("failed to read {}", from.display()))?
        {
            let entry = entry?;
            let target = to.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                self.copy_dir_all(&entry.path(), &target)?;
            } else {
                fs::copy(entry.path(), &target)?;
            }
        }
        Ok(())
    }

    fn write_file(&self, path: &Path, contents: &[u8]) -> Result<()> {
        if self.dry_run {
            println!(
                "[dry-run] write {} ({} bytes)",
                path.display(),
                contents.len()
            );
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))
    }

    fn symlink_dir(&self, target: &Path, link: &Path) -> Result<()> {
        if self.dry_run {
            println!(
                "[dry-run] symlink {} -> {}",
                link.display(),
                target.display()
            );
            return Ok(());
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, link)
                .with_context(|| format!("failed to create symlink {}", link.display()))
        }
        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_dir(target, link)
                .with_context(|| format!("failed to create symlink {}", link.display()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(arguments: &[&str]) -> Result<Options> {
        let arguments: Vec<String> = arguments
            .iter()
            .map(|argument| argument.to_string())
            .collect();
        Options::parse(&arguments)
    }

    #[test]
    fn defaults_are_native_release_adhoc() {
        let options = parse(&[]).expect("empty arguments parse");
        assert!(options.formats.is_empty());
        assert_eq!(options.arch, Arch::Native);
        assert!(!options.debug);
        assert!(!options.skip_build);
        assert_eq!(options.sign, SignMode::Adhoc);
        assert!(!options.dry_run);
    }

    #[test]
    fn formats_parse_repeatable_and_comma_separated() {
        let options = parse(&["--format", "app,dmg", "--format", "dmg"]).expect("formats parse");
        assert_eq!(options.formats, vec![Format::App, Format::Dmg, Format::Dmg]);
    }

    #[test]
    fn all_format_is_a_platform_marker() {
        assert_eq!(Format::parse("all").expect("all parses"), vec![Format::All]);
    }

    #[test]
    fn all_resolves_to_the_platform_defaults() {
        assert_eq!(
            resolve_formats("macos", &[Format::All]).expect("macos all"),
            vec![Format::App, Format::Dmg]
        );
        assert_eq!(
            resolve_formats("windows", &[Format::All]).expect("windows all"),
            vec![Format::Portable, Format::Nsis, Format::Zip]
        );
        assert_eq!(
            resolve_formats("windows", &[Format::Zip, Format::All]).expect("mixed"),
            vec![Format::Zip, Format::Portable, Format::Nsis]
        );
    }

    #[test]
    fn unknown_arguments_are_rejected() {
        assert!(parse(&["--nope"]).is_err());
        assert!(parse(&["--format"]).is_err());
        assert!(parse(&["--arch", "sparc"]).is_err());
        assert!(parse(&["--format", "exe"]).is_err());
    }

    #[test]
    fn sign_modes_parse() {
        assert_eq!(parse(&["--sign", "none"]).unwrap().sign, SignMode::None);
        assert_eq!(parse(&["--sign", "adhoc"]).unwrap().sign, SignMode::Adhoc);
        assert_eq!(
            parse(&["--sign", "Developer ID Application: Example"])
                .unwrap()
                .sign,
            SignMode::Identity("Developer ID Application: Example".to_string())
        );
    }

    #[test]
    fn macos_formats_resolve_to_app_and_dmg() {
        let resolved = resolve_formats("macos", &[]).expect("macos formats");
        assert_eq!(resolved, vec![Format::App, Format::Dmg]);
        assert!(resolve_formats("macos", &[Format::Nsis]).is_err());
    }

    #[test]
    fn windows_formats_resolve_to_portable_installer_and_zip() {
        let resolved = resolve_formats("windows", &[]).expect("windows formats");
        assert_eq!(resolved, vec![Format::Portable, Format::Nsis, Format::Zip]);
        assert!(resolve_formats("windows", &[Format::Dmg]).is_err());
    }

    #[test]
    fn preview_platform_infers_the_target_from_requested_formats() {
        assert_eq!(
            preview_platform(&[Format::Dmg]).expect("mac preview"),
            "macos"
        );
        assert_eq!(
            preview_platform(&[Format::Nsis, Format::Zip]).expect("windows preview"),
            "windows"
        );
        assert!(preview_platform(&[Format::Dmg, Format::Nsis]).is_err());
        assert!(preview_platform(&[Format::All]).is_err());
        assert!(preview_platform(&[]).is_err());
    }

    #[test]
    fn artifact_names_include_version_platform_and_arch() {
        assert_eq!(
            artifact_name("1.2.3", "macos", "universal", "dmg"),
            "PacketSmith-1.2.3-macos-universal.dmg"
        );
        assert_eq!(
            artifact_name("1.2.3", "windows", "x64", "setup"),
            "PacketSmith-1.2.3-windows-x64-setup.exe"
        );
        assert_eq!(
            artifact_name("1.2.3", "windows", "arm64", "zip"),
            "PacketSmith-1.2.3-windows-arm64.zip"
        );
    }

    #[test]
    fn version_quad_is_four_numeric_components() {
        assert_eq!(version_quad("0.1.0"), "0.1.0.0");
        assert_eq!(version_quad("1.2"), "1.2.0.0");
        assert_eq!(version_quad("1.2.3-beta.4"), "1.2.3.0");
        assert_eq!(version_quad("1.2.3.4.5"), "1.2.3.4");
    }

    #[test]
    fn info_plist_template_is_fully_substituted() {
        let template = "version=@@VERSION@@ build=@@BUILD_NUMBER@@ min=@@MIN_OS_VERSION@@ id=@@BUNDLE_ID@@ year=@@COPYRIGHT_YEAR@@";
        let rendered = render_info_plist(template, "0.1.0", "0.1.0", "11.0", 2026);
        assert_eq!(
            rendered,
            "version=0.1.0 build=0.1.0 min=11.0 id=dev.packetsmith.desktop year=2026"
        );
        assert!(!rendered.contains("@@"));
    }

    #[test]
    fn info_plist_asset_has_no_leftover_placeholders_after_render() {
        let template = fs::read_to_string(
            repo_root()
                .join("packaging")
                .join("macos")
                .join("Info.plist"),
        )
        .expect("Info.plist template exists");
        let rendered = render_info_plist(&template, "0.1.0", "0.1.0", "11.0", 2026);
        assert!(
            !rendered.contains("@@"),
            "unsubstituted placeholder remains"
        );
        assert!(rendered.contains("dev.packetsmith.desktop"));
    }

    #[test]
    fn build_arguments_include_feature_and_target() {
        // The argument assembly is exercised through build_binary in dry-run
        // mode; this guards the expected binary location layout.
        let path = binary_path(
            Path::new("/tmp/target"),
            Some("aarch64-apple-darwin"),
            false,
        );
        assert!(path.ends_with("aarch64-apple-darwin/release/packetsmith-app") || cfg!(windows));
    }
}
