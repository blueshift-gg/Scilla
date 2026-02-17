use {
    crate::{
        commands::CommandFlow,
        misc::helpers::command_exists,
        prompt::{prompt_build_mode, prompt_input_data, prompt_select_data},
        ui::show_spinner,
    },
    anyhow::{anyhow, bail},
    console::style,
    std::{
        env, fmt, fs,
        path::{Path, PathBuf},
        process::Command as ProcessCommand,
    },
    toml::Table,
};

const SBPF_BUILD_CONFIG_TOML: &str = r#"[unstable]
build-std = ["core", "alloc"]
[target.bpfel-unknown-none]
rustflags = [
    "-C", "linker=sbpf-linker",
    "-C", "panic=abort",
    "-C", "save-temps",
    "-C", "link-arg=--llvm-args=-bpf-stack-size=4096",
    "-C", "relocation-model=static",
    "-A", "unexpected_cfgs",
]

[alias]
build-bpf = "build --release --target bpfel-unknown-none"
"#;

pub async fn process_build() -> anyhow::Result<CommandFlow> {
    println!(
        "{}",
        style("This command will expand configs and build the program for sbpf target")
            .yellow()
            .dim()
    );
    let program_dir = resolve_program_dir()?;
    let build_mode = prompt_build_mode()?;

    let build_context = resolve_build_context(&program_dir)?;

    if build_mode == BuildMode::Upstream {
        prepare_upstream_build(&build_context)?;
        show_spinner(
            "Building program for sbpf target...",
            run_upstream_build(&build_context),
        )
        .await;
        print_build_output(&build_context, build_mode);
    }

    if build_mode == BuildMode::Solana {
        show_spinner(
            "Building program for sbpf target...",
            run_normal_build(&build_context),
        )
        .await;
    }

    Ok(CommandFlow::Processed)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildMode {
    Upstream,
    Solana,
}

impl fmt::Display for BuildMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            BuildMode::Upstream => "Upstream (sbpf-linker)",
            BuildMode::Solana => "Solana (cargo build-sbf)",
        };
        write!(f, "{label}")
    }
}

#[derive(Debug, Clone)]
struct BuildContext {
    program_dir: PathBuf,
    workspace_root: Option<PathBuf>,
    package_name: String,
}

/// Parsed Cargo.toml manifest.
struct Manifest {
    table: toml::Table,
}

impl Manifest {
    fn from_path(path: &Path) -> anyhow::Result<Self> {
        let raw = fs::read_to_string(path)?;
        let table: Table =
            toml::from_str(&raw).map_err(|e| anyhow!("Failed to parse {}: {e}", path.display()))?;
        Ok(Self { table })
    }

    fn try_from_program_dir(dir: &Path) -> Option<Self> {
        let path = dir.join("Cargo.toml");
        if !path.is_file() {
            return None;
        }
        Self::from_path(&path).ok()
    }

    fn has_section(&self, section: &str) -> bool {
        self.table.get(section).is_some()
    }

    fn package_name(&self) -> Option<String> {
        self.table
            .get("package")?
            .get("name")?
            .as_str()
            .map(String::from)
    }

    fn workspace_members(&self) -> Vec<String> {
        self.table
            .get("workspace")
            .and_then(|w| w.get("members"))
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .filter(|s| !s.contains('*') && !s.contains('{'))
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default()
    }
}

fn prepare_upstream_build(build_context: &BuildContext) -> anyhow::Result<&BuildContext> {
    ensure_sbpf_linker()?;
    ensure_llvm()?;
    expand_repo_config(build_context)?;
    Ok(build_context)
}

fn resolve_build_context(program_dir: &Path) -> anyhow::Result<BuildContext> {
    let manifest_path = program_dir.join("Cargo.toml");
    let manifest = Manifest::from_path(&manifest_path)?;
    let mut is_workspace = false;
    let mut is_package = false;

    if manifest.has_section("workspace") {
        is_workspace = true;
    }
    if manifest.has_section("package") {
        is_package = true;
    }

    if is_package {
        resolve_package_build_context(program_dir, &manifest)
    } else if is_workspace {
        resolve_workspace_build_context(program_dir)
    } else {
        bail!("Cargo.toml is missing [workspace] or [package] section")
    }
}

fn resolve_workspace_build_context(workspace_root: &Path) -> anyhow::Result<BuildContext> {
    let (program_dir, package_name) = prompt_workspace_member(workspace_root)?;
    Ok(BuildContext {
        program_dir,
        workspace_root: Some(workspace_root.to_path_buf()),
        package_name,
    })
}

fn resolve_package_build_context(
    program_dir: &Path,
    manifest: &Manifest,
) -> anyhow::Result<BuildContext> {
    let package_name = manifest.package_name().ok_or_else(|| {
        anyhow!(
            "Failed to read package name from {}",
            program_dir.join("Cargo.toml").display()
        )
    })?;
    let workspace_root = find_workspace_root(program_dir)?;

    Ok(BuildContext {
        program_dir: program_dir.to_path_buf(),
        workspace_root,
        package_name,
    })
}

async fn run_upstream_build(build_context: &BuildContext) -> anyhow::Result<()> {
    let mut command = ProcessCommand::new("cargo");
    command.arg("+nightly");
    command.arg("build-bpf");

    if let Some(workspace_root) = &build_context.workspace_root {
        command.args(["-p", &build_context.package_name]);
        command.current_dir(workspace_root);
    } else {
        command.current_dir(&build_context.program_dir);
    }

    let output = command
        .output()
        .map_err(|err| anyhow!("Failed to run cargo +nightly build-bpf: {err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(anyhow!(
            "cargo +nightly build-bpf failed. stdout: {stdout}\nstderr: {stderr}",
        ));
    }

    Ok(())
}

async fn run_normal_build(build_context: &BuildContext) -> anyhow::Result<()> {
    ensure_cargo_sbf()?;
    let mut command = ProcessCommand::new("cargo");
    command.arg("build-sbf");

    if let Some(workspace_root) = &build_context.workspace_root {
        command.args(["--", "-p", &build_context.package_name]);
        command.current_dir(workspace_root);
    } else {
        command.current_dir(&build_context.program_dir);
    }

    let output = command
        .output()
        .map_err(|err| anyhow!("Failed to run cargo build-sbf: {err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(anyhow!(
            "cargo build-sbf failed. stdout: {stdout}\nstderr: {stderr}",
        ));
    }

    Ok(())
}

fn print_build_output(build_context: &BuildContext, build_mode: BuildMode) {
    let build_root = build_context
        .workspace_root
        .as_deref()
        .unwrap_or(&build_context.program_dir);
    let lib_name = build_context.package_name.replace('-', "_");
    let package_name = &build_context.package_name;
    let target_dir = build_root.join("target");

    let primary_output = match build_mode {
        BuildMode::Upstream => target_dir
            .join("bpfel-unknown-none/release")
            .join(format!("lib{lib_name}.so")),
        BuildMode::Solana => target_dir.join("deploy").join(format!("{package_name}.so")),
    };

    let display_path = relative_display_path(build_root, &primary_output);
    println!("{} {}", style("Build output:").green().bold(), display_path);
}

fn resolve_program_dir() -> anyhow::Result<PathBuf> {
    let current_dir = env::current_dir()?;
    if is_package_dir(&current_dir) || is_workspace_dir(&current_dir) {
        return Ok(current_dir);
    }

    let program_dir: PathBuf =
        prompt_input_data("Enter the path to the program directory to build for sbpf target:");
    if !is_package_dir(&program_dir) && !is_workspace_dir(&program_dir) {
        return Err(anyhow!("No Cargo.toml found in {}", program_dir.display()));
    }

    Ok(program_dir)
}

fn find_workspace_root(program_dir: &Path) -> anyhow::Result<Option<PathBuf>> {
    for ancestor in program_dir.ancestors() {
        let manifest_path = ancestor.join("Cargo.toml");
        if !manifest_path.is_file() {
            continue;
        }
        let manifest = Manifest::from_path(&manifest_path)?;
        if manifest.has_section("workspace") {
            return Ok(Some(ancestor.to_path_buf()));
        }
    }

    Ok(None)
}

fn prompt_workspace_member(workspace_root: &Path) -> anyhow::Result<(PathBuf, String)> {
    let manifest = Manifest::from_path(&workspace_root.join("Cargo.toml"))?;
    let members = manifest.workspace_members();
    let candidate_dir = if members.is_empty() {
        let member_path: PathBuf = prompt_input_data(
            "Workspace detected. Enter the relative path to the package to build:",
        );
        if member_path.is_absolute() {
            member_path
        } else {
            workspace_root.join(member_path)
        }
    } else {
        println!(
            "{} {}",
            style("Workspace detected:").yellow().bold(),
            workspace_root.display()
        );
        let selection = prompt_select_data("Select the package to build:", members);
        workspace_root.join(&selection)
    };

    let package_name = read_package_name(&candidate_dir)?;
    Ok((candidate_dir, package_name))
}

fn read_package_name(program_dir: &Path) -> anyhow::Result<String> {
    let manifest_path = program_dir.join("Cargo.toml");
    Manifest::from_path(&manifest_path)?
        .package_name()
        .ok_or_else(|| {
            anyhow!(
                "Failed to read package name from {}",
                manifest_path.display()
            )
        })
}

fn expand_repo_config(build_context: &BuildContext) -> anyhow::Result<()> {
    ensure_cargo_config(&build_context.program_dir)?;
    ensure_program_manifest(&build_context.program_dir)?;
    let manifest_root = build_context
        .workspace_root
        .as_deref()
        .unwrap_or_else(|| &build_context.program_dir);
    ensure_workspace_manifest(manifest_root)?;
    Ok(())
}

fn ensure_cargo_config(config_root: &Path) -> anyhow::Result<()> {
    let config_dir = config_root.join(".cargo");
    let config_path = config_dir.join("config.toml");
    fs::create_dir_all(&config_dir)?;
    fs::write(&config_path, SBPF_BUILD_CONFIG_TOML)?;
    Ok(())
}

fn ensure_program_manifest(program_dir: &Path) -> anyhow::Result<()> {
    let manifest_path = program_dir.join("Cargo.toml");
    update_manifest(&manifest_path, |lines| {
        ensure_section_entries(lines, "[lib]", &["crate-type = [\"cdylib\"]"]);
    })
}

fn ensure_workspace_manifest(workspace_root: &Path) -> anyhow::Result<()> {
    let manifest_path = workspace_root.join("Cargo.toml");
    update_manifest(&manifest_path, |lines| {
        ensure_section_entries(
            lines,
            "[profile.release]",
            &[
                "overflow-checks = true",
                "lto = \"fat\"",
                "codegen-units = 1",
            ],
        );
        ensure_section_entries(
            lines,
            "[profile.release.build-override]",
            &["opt-level = 3", "incremental = false", "codegen-units = 1"],
        );
    })
}

fn update_manifest<F>(manifest_path: &Path, modifier: F) -> anyhow::Result<()>
where
    F: FnOnce(&mut Vec<String>),
{
    let contents = fs::read_to_string(manifest_path)?;
    let mut lines: Vec<String> = contents.lines().map(String::from).collect();
    modifier(&mut lines);
    let mut output = lines.join("\n");
    if contents.ends_with('\n') {
        output.push('\n');
    }
    fs::write(manifest_path, output)?;
    Ok(())
}

fn ensure_section_entries(lines: &mut Vec<String>, header: &str, entries: &[&str]) {
    let (start, end) = find_section(lines, header);
    if let Some(start_idx) = start {
        let mut insert_at = end;
        for entry in entries {
            let key = entry.split('=').next().unwrap_or("").trim();
            if !section_has_key(lines, start_idx + 1, end, key) {
                lines.insert(insert_at, entry.to_string());
                insert_at += 1;
            }
        }
        return;
    }

    if !lines.is_empty() && !lines.last().unwrap().trim().is_empty() {
        lines.push(String::new());
    }
    lines.push(header.to_string());
    for entry in entries {
        lines.push(entry.to_string());
    }
}

fn find_section(lines: &[String], header: &str) -> (Option<usize>, usize) {
    let mut start = None;
    let mut end = lines.len();

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if !(trimmed.starts_with('[') && trimmed.ends_with(']')) {
            continue;
        }
        if start.is_some() {
            end = idx;
            break;
        }
        if trimmed == header {
            start = Some(idx);
        }
    }

    (start, end)
}

fn section_has_key(lines: &[String], start: usize, end: usize, key: &str) -> bool {
    lines[start..end].iter().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with(key) && trimmed.contains('=')
    })
}

fn ensure_sbpf_linker() -> anyhow::Result<()> {
    if command_exists("sbpf-linker") {
        return Ok(());
    }

    Err(anyhow!(
        "sbpf-linker is required. Install it with: cargo install sbpf-linker"
    ))
}

fn ensure_llvm() -> anyhow::Result<()> {
    if command_exists("llvm-config") {
        return Ok(());
    }

    bail!("LLVM is required. Install it manually and ensure llvm-config is on PATH.")
}

fn ensure_cargo_sbf() -> anyhow::Result<()> {
    if command_exists("cargo-build-sbf") {
        return Ok(());
    }

    bail!(
        "cargo build-sbf is required. Install solana-cargo-build-sbf and ensure cargo-build-sbf \
         is on PATH."
    )
}

fn is_package_dir(path: &Path) -> bool {
    Manifest::try_from_program_dir(path).is_some_and(|m| m.has_section("package"))
}

fn is_workspace_dir(path: &Path) -> bool {
    Manifest::try_from_program_dir(path).is_some_and(|m| m.has_section("workspace"))
}

fn relative_display_path(root: &Path, path: &Path) -> String {
    let root_path = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let target_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    match target_path.strip_prefix(&root_path) {
        Ok(relative) => format!("./{}", relative.display()),
        Err(_) => path.display().to_string(),
    }
}
