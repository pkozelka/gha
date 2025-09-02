use serde::Deserialize;
use std::{fs, path::{Path, PathBuf}, io::Write};
use anyhow::{Result, Context};
use crate::git_utils;
use crate::git_utils::RefInfo;

#[derive(Debug, Deserialize)]
struct Workflow {
    #[serde(default)]
    on: WorkflowOn,
}

#[derive(Debug, Deserialize, Default)]
struct WorkflowOn {
    #[serde(rename = "workflow_dispatch", default)]
    workflow_dispatch: Option<WorkflowDispatch>,
}

#[derive(Debug, Deserialize)]
struct WorkflowDispatch {
    #[serde(default)]
    inputs: std::collections::HashMap<String, WorkflowInput>,
}

#[derive(Debug, Deserialize)]
struct WorkflowInput {
    description: Option<String>,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    r#type: Option<String>, // e.g. "choice"
    #[serde(default)]
    options: Option<Vec<String>>,
}

/// Entry point: generate Makefile for workflow_dispatch clients
pub fn generate_makefile(output: &Path) -> Result<()> {
    let workflows = discover_workflows(".github/workflows")?;

    let mut out = fs::File::create(output)
        .with_context(|| format!("failed to create output file {}", output.display()))?;

    generate_makefile_header(&mut out)?;

    for wf_path in workflows {
        if let Some(dispatch) = parse_workflow(&wf_path)? {
            generate_workflow_targets(&mut out, &wf_path, dispatch)?;
        }
    }

    Ok(())
}

/// Find all YAML workflow files in given directory
fn discover_workflows(dir: &str) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let path = Path::new(dir);

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext == "yml" || ext == "yaml" {
                    files.push(path);
                }
            }
        }
    }

    Ok(files)
}

/// Parse workflow file, return workflow_dispatch if present
fn parse_workflow(path: &Path) -> Result<Option<WorkflowDispatch>> {
    let text = fs::read_to_string(path)?;
    let workflow: Workflow = serde_yaml::from_str(&text)
        .with_context(|| format!("failed to parse {}", path.display()))?;

    Ok(workflow.on.workflow_dispatch)
}

/// Write header of Makefile: variables and curl macro
fn generate_makefile_header(out: &mut fs::File) -> Result<()> {
    let repo = git_utils::default_repo_from_git()
        .map(|r| format!("{}/{}", r.owner, r.repo))
        .unwrap_or_else(|| "<owner>/<repo>".into());

    let reference = git_utils::default_ref_from_git().unwrap_or_else(|| RefInfo::new("main".to_string()));

    writeln!(out, "GITHUB_TOKEN ?=")?;
    writeln!(out, "REPO ?={repo}")?;
    writeln!(out, "REF ?={reference}")?;
    writeln!(out)?;

    writeln!(out, "CURL_BASE = curl -sSL -H \"Accept: application/vnd.github+json\" \\")?;
    writeln!(out, "\t-H \"Authorization: Bearer $(GITHUB_TOKEN)\" \\")?;
    writeln!(out, "\t-H \"X-GitHub-Api-Version: 2022-11-28\"")?;
    writeln!(out)?;

    writeln!(out, "define DISPATCH")?;
    writeln!(out, "\t$(CURL_BASE) -X POST \\")?;
    writeln!(out, "\t  https://api.github.com/repos/$$(REPO)/actions/workflows/$$1/dispatches \\")?;
    writeln!(out, "\t  -d \"$$2\"")?;
    writeln!(out, "endef")?;
    writeln!(out)?;

    Ok(())
}

/// Generate one or more targets for a workflow
fn generate_workflow_targets(out: &mut fs::File, path: &Path, dispatch: WorkflowDispatch) -> Result<()> {
    let wf_name = path.file_name().unwrap().to_string_lossy();

    // If first input is a choice → one target per option
    if let Some((first_name, first_input)) = dispatch.inputs.iter().next() {
        if first_input.r#type.as_deref() == Some("choice") {
            if let Some(options) = &first_input.options {
                for opt in options {
                    generate_target(out, &wf_name, &dispatch, Some((first_name, opt)))?;
                }
                return Ok(());
            }
        }
    }

    // Default: one target
    generate_target(out, &wf_name, &dispatch, None)
}

/// Generate a single Make target
fn generate_target(
    out: &mut fs::File,
    wf_name: &str,
    dispatch: &WorkflowDispatch,
    choice: Option<(&String, &String)>,
) -> Result<()> {
    let mut target = wf_name.to_string();
    if let Some((_, opt)) = &choice {
        target.push('-');
        target.push_str(&opt.to_uppercase());
    }

    writeln!(out, "{target}:")?;

    // Required input checks
    for (name, input) in &dispatch.inputs {
        if input.required {
            let var = name.to_uppercase();
            writeln!(out, "\t@test -n \"$({var})\" || (echo \"{var} is required\"; exit 1)")?;
        }
    }

    let payload = make_payload(dispatch, choice);

    writeln!(out, "\t$$(call DISPATCH,{wf_name},{payload})")?;
    writeln!(out)?;

    Ok(())
}

/// Build JSON payload string for curl
fn make_payload(dispatch: &WorkflowDispatch, choice: Option<(&String, &String)>) -> String {
    let mut payload = String::from("{\\\"ref\\\":\\\"$$(REF)\\\",\\\"inputs\\\":{");

    let mut first = true;
    for (name, _) in &dispatch.inputs {
        if !first {
            payload.push(',');
        }
        first = false;

        if let Some((choice_name, opt)) = &choice {
            if name == *choice_name {
                payload.push_str(&format!("\\\"{name}\\\":\\\"{opt}\\\""));
                continue;
            }
        }

        let var = name.to_uppercase();
        payload.push_str(&format!("\\\"{name}\\\":\\\"$({var})\\\""));
    }
    payload.push_str("}}");

    payload
}
