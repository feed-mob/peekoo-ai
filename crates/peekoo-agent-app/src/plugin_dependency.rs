use std::collections::HashMap;
use std::process::Command;

use peekoo_plugin_host::{PluginManifest, RuntimeDependencyDef, RuntimeDependencyKind};
use peekoo_plugin_store::{
    DependencyCheckStatusDto, PluginDependencySummaryDto, RuntimeDependencyStatusDto,
};
use regex::Regex;
use semver::Version;

pub fn summarize_manifest_dependencies(manifest: &PluginManifest) -> PluginDependencySummaryDto {
    let mut checker = RuntimeDependencyChecker::default();
    checker.summarize(manifest)
}

pub fn first_blocking_dependency_message(summary: &PluginDependencySummaryDto) -> Option<String> {
    summary
        .dependencies
        .iter()
        .find(|dependency| {
            dependency.required && dependency.status != DependencyCheckStatusDto::Satisfied
        })
        .map(format_dependency_message)
}

fn format_dependency_message(dependency: &RuntimeDependencyStatusDto) -> String {
    match dependency.status {
        DependencyCheckStatusDto::Missing => match &dependency.min_version {
            Some(min_version) => format!(
                "{} {}+ is required; {} was not found",
                dependency.display_name,
                min_version,
                dependency
                    .command_tried
                    .as_deref()
                    .unwrap_or("required command")
            ),
            None => format!(
                "{} is required; {} was not found",
                dependency.display_name,
                dependency
                    .command_tried
                    .as_deref()
                    .unwrap_or("required command")
            ),
        },
        DependencyCheckStatusDto::VersionMismatch => format!(
            "{} {}+ is required; detected {}",
            dependency.display_name,
            dependency
                .min_version
                .as_deref()
                .unwrap_or("required version"),
            dependency
                .detected_version
                .as_deref()
                .unwrap_or("unknown version")
        ),
        DependencyCheckStatusDto::Unknown => format!(
            "{} is required; version could not be verified",
            dependency.display_name
        ),
        DependencyCheckStatusDto::Satisfied => dependency
            .message
            .clone()
            .unwrap_or_else(|| format!("{} is available", dependency.display_name)),
    }
}

#[derive(Debug, Clone)]
struct CommandCandidate {
    executable: String,
    args: Vec<String>,
    display: String,
}

#[derive(Default)]
struct RuntimeDependencyChecker {
    cache: HashMap<String, RuntimeDependencyStatusDto>,
}

impl RuntimeDependencyChecker {
    fn summarize(&mut self, manifest: &PluginManifest) -> PluginDependencySummaryDto {
        let mut dependencies = Vec::new();
        let mut blocking_issues = 0;
        let mut warnings = 0;

        for dependency in &manifest.runtime_dependencies {
            if !dependency_applies_to_current_platform(dependency) {
                continue;
            }

            let status = self.check_dependency(dependency);
            let is_satisfied = status.status == DependencyCheckStatusDto::Satisfied;

            if !is_satisfied && status.required {
                blocking_issues += 1;
            } else if !is_satisfied {
                warnings += 1;
            }

            dependencies.push(status);
        }

        PluginDependencySummaryDto {
            has_required_dependencies: blocking_issues == 0,
            blocking_issues,
            warnings,
            dependencies,
        }
    }

    fn check_dependency(
        &mut self,
        dependency: &RuntimeDependencyDef,
    ) -> RuntimeDependencyStatusDto {
        let required = dependency.required.unwrap_or(true);
        let candidates = command_candidates(dependency);
        if candidates.is_empty() {
            return RuntimeDependencyStatusDto {
                kind: dependency_kind_key(&dependency.kind).to_string(),
                required,
                display_name: dependency_display_name(dependency),
                command_tried: dependency.command.clone(),
                status: if required {
                    DependencyCheckStatusDto::Unknown
                } else {
                    DependencyCheckStatusDto::Satisfied
                },
                detected_version: None,
                min_version: dependency.min_version.clone(),
                message: Some("No runtime check command configured".to_string()),
                install_hint: dependency.install_hint.clone(),
            };
        }

        let mut last_missing = None;

        for candidate in candidates {
            let cache_key = format!("{}::{}", candidate.executable, candidate.args.join(" "));

            let base_status = if let Some(cached) = self.cache.get(&cache_key) {
                cached.clone()
            } else {
                let evaluated = evaluate_candidate(&candidate, dependency);
                self.cache.insert(cache_key, evaluated.clone());
                evaluated
            };

            if base_status.status == DependencyCheckStatusDto::Missing {
                last_missing = Some(base_status);
                continue;
            }

            return apply_version_requirement(base_status, dependency, required);
        }

        let mut status = last_missing.unwrap_or_else(|| missing_status(dependency, required));
        status.required = required;
        status.min_version = dependency.min_version.clone();
        status.install_hint = dependency.install_hint.clone();
        status.display_name = dependency_display_name(dependency);
        status
    }
}

fn evaluate_candidate(
    candidate: &CommandCandidate,
    dependency: &RuntimeDependencyDef,
) -> RuntimeDependencyStatusDto {
    let required = dependency.required.unwrap_or(true);
    let display_name = dependency_display_name(dependency);
    let kind = dependency_kind_key(&dependency.kind).to_string();
    let command_tried = Some(candidate.display.clone());

    let output = Command::new(&candidate.executable)
        .args(&candidate.args)
        .output();

    match output {
        Ok(output) => {
            let combined = join_output(&output.stdout, &output.stderr);
            let detected_version = extract_version(&combined);

            RuntimeDependencyStatusDto {
                kind,
                required,
                display_name,
                command_tried,
                status: DependencyCheckStatusDto::Satisfied,
                detected_version,
                min_version: dependency.min_version.clone(),
                message: None,
                install_hint: dependency.install_hint.clone(),
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => RuntimeDependencyStatusDto {
            kind,
            required,
            display_name,
            command_tried,
            status: DependencyCheckStatusDto::Missing,
            detected_version: None,
            min_version: dependency.min_version.clone(),
            message: None,
            install_hint: dependency.install_hint.clone(),
        },
        Err(error) => RuntimeDependencyStatusDto {
            kind,
            required,
            display_name,
            command_tried,
            status: DependencyCheckStatusDto::Unknown,
            detected_version: None,
            min_version: dependency.min_version.clone(),
            message: Some(error.to_string()),
            install_hint: dependency.install_hint.clone(),
        },
    }
}

fn apply_version_requirement(
    mut status: RuntimeDependencyStatusDto,
    dependency: &RuntimeDependencyDef,
    required: bool,
) -> RuntimeDependencyStatusDto {
    status.required = required;
    status.min_version = dependency.min_version.clone();
    status.install_hint = dependency.install_hint.clone();
    status.display_name = dependency_display_name(dependency);

    let Some(min_version) = dependency.min_version.as_deref() else {
        return status;
    };

    let Some(detected_version) = status.detected_version.as_deref() else {
        status.status = if required {
            DependencyCheckStatusDto::Unknown
        } else {
            DependencyCheckStatusDto::Satisfied
        };
        return status;
    };

    let detected = parse_version(detected_version);
    let required_version = parse_version(min_version);

    match (detected, required_version) {
        (Some(detected), Some(required_version)) if detected < required_version => {
            status.status = if required {
                DependencyCheckStatusDto::VersionMismatch
            } else {
                DependencyCheckStatusDto::Satisfied
            };
            status
        }
        (Some(_), Some(_)) => status,
        _ => {
            status.status = if required {
                DependencyCheckStatusDto::Unknown
            } else {
                DependencyCheckStatusDto::Satisfied
            };
            status
        }
    }
}

fn missing_status(dependency: &RuntimeDependencyDef, required: bool) -> RuntimeDependencyStatusDto {
    RuntimeDependencyStatusDto {
        kind: dependency_kind_key(&dependency.kind).to_string(),
        required,
        display_name: dependency_display_name(dependency),
        command_tried: dependency.command.clone(),
        status: DependencyCheckStatusDto::Missing,
        detected_version: None,
        min_version: dependency.min_version.clone(),
        message: None,
        install_hint: dependency.install_hint.clone(),
    }
}

fn dependency_display_name(dependency: &RuntimeDependencyDef) -> String {
    dependency
        .display_name
        .clone()
        .unwrap_or_else(|| default_display_name(&dependency.kind).to_string())
}

fn dependency_kind_key(kind: &RuntimeDependencyKind) -> &'static str {
    match kind {
        RuntimeDependencyKind::Python => "python",
        RuntimeDependencyKind::Node => "node",
        RuntimeDependencyKind::Dotnet => "dotnet",
        RuntimeDependencyKind::Ruby => "ruby",
        RuntimeDependencyKind::Rust => "rust",
        RuntimeDependencyKind::Java => "java",
        RuntimeDependencyKind::Custom => "custom",
    }
}

fn default_display_name(kind: &RuntimeDependencyKind) -> &'static str {
    match kind {
        RuntimeDependencyKind::Python => "Python",
        RuntimeDependencyKind::Node => "Node.js",
        RuntimeDependencyKind::Dotnet => ".NET",
        RuntimeDependencyKind::Ruby => "Ruby",
        RuntimeDependencyKind::Rust => "Rust",
        RuntimeDependencyKind::Java => "Java",
        RuntimeDependencyKind::Custom => "Runtime dependency",
    }
}

fn dependency_applies_to_current_platform(dependency: &RuntimeDependencyDef) -> bool {
    if dependency.platforms.is_empty() {
        return true;
    }

    dependency
        .platforms
        .iter()
        .any(|platform| platform.eq_ignore_ascii_case(std::env::consts::OS))
}

fn command_candidates(dependency: &RuntimeDependencyDef) -> Vec<CommandCandidate> {
    if let Some(command) = dependency.command.as_ref() {
        return vec![CommandCandidate {
            executable: command.clone(),
            args: default_version_args(&dependency.kind)
                .into_iter()
                .map(str::to_string)
                .collect(),
            display: format!(
                "{} {}",
                command,
                default_version_args(&dependency.kind).join(" ")
            )
            .trim()
            .to_string(),
        }];
    }

    match dependency.kind {
        RuntimeDependencyKind::Python => {
            #[cfg(target_os = "windows")]
            {
                vec![
                    CommandCandidate {
                        executable: "py".to_string(),
                        args: vec!["-3".to_string(), "--version".to_string()],
                        display: "py -3 --version".to_string(),
                    },
                    CommandCandidate {
                        executable: "python".to_string(),
                        args: vec!["--version".to_string()],
                        display: "python --version".to_string(),
                    },
                    CommandCandidate {
                        executable: "python3".to_string(),
                        args: vec!["--version".to_string()],
                        display: "python3 --version".to_string(),
                    },
                ]
            }
            #[cfg(not(target_os = "windows"))]
            {
                vec![
                    CommandCandidate {
                        executable: "python3".to_string(),
                        args: vec!["--version".to_string()],
                        display: "python3 --version".to_string(),
                    },
                    CommandCandidate {
                        executable: "python".to_string(),
                        args: vec!["--version".to_string()],
                        display: "python --version".to_string(),
                    },
                ]
            }
        }
        RuntimeDependencyKind::Node => vec![CommandCandidate {
            executable: "node".to_string(),
            args: vec!["--version".to_string()],
            display: "node --version".to_string(),
        }],
        RuntimeDependencyKind::Dotnet => vec![CommandCandidate {
            executable: "dotnet".to_string(),
            args: vec!["--version".to_string()],
            display: "dotnet --version".to_string(),
        }],
        RuntimeDependencyKind::Ruby => vec![CommandCandidate {
            executable: "ruby".to_string(),
            args: vec!["--version".to_string()],
            display: "ruby --version".to_string(),
        }],
        RuntimeDependencyKind::Rust => vec![CommandCandidate {
            executable: "rustc".to_string(),
            args: vec!["--version".to_string()],
            display: "rustc --version".to_string(),
        }],
        RuntimeDependencyKind::Java => vec![CommandCandidate {
            executable: "java".to_string(),
            args: vec!["-version".to_string()],
            display: "java -version".to_string(),
        }],
        RuntimeDependencyKind::Custom => Vec::new(),
    }
}

fn default_version_args(kind: &RuntimeDependencyKind) -> Vec<&'static str> {
    match kind {
        RuntimeDependencyKind::Java => vec!["-version"],
        RuntimeDependencyKind::Custom => Vec::new(),
        _ => vec!["--version"],
    }
}

fn join_output(stdout: &[u8], stderr: &[u8]) -> String {
    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(stdout));
    if !stderr.is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&String::from_utf8_lossy(stderr));
    }
    text
}

fn extract_version(text: &str) -> Option<String> {
    let regex = Regex::new(r"(\d+\.\d+(?:\.\d+)?)").ok()?;
    regex
        .captures(text)
        .and_then(|captures| captures.get(1))
        .map(|capture| capture.as_str().to_string())
}

fn parse_version(value: &str) -> Option<Version> {
    let parts: Vec<&str> = value.trim().trim_start_matches('v').split('.').collect();
    if parts.is_empty() {
        return None;
    }

    let mut normalized = parts;
    while normalized.len() < 3 {
        normalized.push("0");
    }

    let normalized = normalized.into_iter().take(3).collect::<Vec<_>>().join(".");
    Version::parse(&normalized).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use peekoo_plugin_host::manifest::parse_manifest;

    #[test]
    fn summary_marks_java_missing_when_command_is_unavailable() {
        let manifest = parse_manifest(
            r#"
[plugin]
key = "example"
name = "Example"
version = "0.1.0"
wasm = "plugin.wasm"

[[runtime_dependencies]]
kind = "java"
command = "definitely-not-a-real-java-command"
"#,
        )
        .unwrap();

        let summary = summarize_manifest_dependencies(&manifest);
        assert!(!summary.has_required_dependencies);
        assert_eq!(summary.blocking_issues, 1);
        assert_eq!(
            summary.dependencies[0].status,
            DependencyCheckStatusDto::Missing
        );
    }

    #[test]
    fn parse_version_pads_short_versions() {
        assert_eq!(parse_version("3.10"), Some(Version::new(3, 10, 0)));
        assert_eq!(parse_version("21.0.2"), Some(Version::new(21, 0, 2)));
    }
}
