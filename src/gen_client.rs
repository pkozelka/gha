use crate::git_utils;
use anyhow::{Context, Result};
use std::{
    fs,
    path::Path,
};

/// Normalized workflow info
pub struct WorkflowInfo {
    pub file: String,
    pub name: String,
    pub inputs: Vec<InputInfo>,
}

pub struct InputInfo {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
    pub default: Option<String>,
    pub ui_type: Option<String>,
    pub options: Vec<String>,
}

/// Entry point: parse workflows, then write Makefile
pub fn generate_makefile(workflows_dir: &Path, output: &Path) -> Result<()> {
    if !workflows_dir.is_dir() {
        anyhow::bail!("{} is not a directory or does not exist", workflows_dir.display());
    }
    let dir = workflows_dir.canonicalize()?;
    tracing::info!("Discovering workflows in {}", dir.display());
    let workflows = discover_and_parse(&dir)?;

    // Transform to rendering model
    let model = build_render_model(&dir, &workflows)?;

    // Render via template
    let content = render_with_template(&model)?;

    fs::write(output, content)
        .with_context(|| format!("failed to write {}", output.display()))
}

/// Discover YAML workflows and parse them
fn discover_and_parse(path: &Path) -> Result<Vec<WorkflowInfo>> {
    let mut infos = Vec::new();

    if !path.is_dir() {
        return Ok(infos);
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext == "yml" || ext == "yaml" {
                if let Some(info) = parse_workflow(&path)? {
                    infos.push(info);
                }
            }
        }
    }
    tracing::info!("Found {} workflow files in {}", infos.len(), path.display());

    Ok(infos)
}

/// Parse a workflow into WorkflowInfo
pub fn parse_workflow(path: &Path) -> Result<Option<WorkflowInfo>> {
    let yaml = fs::read_to_string(path)?;
    let value: serde_json::Value = serde_yml::from_str(&yaml)
        .with_context(|| format!("failed to parse {}", path.display()))?;

    let on = value.get("on");
    if on.is_none() {
        return Ok(None);
    }
    let on = on.unwrap();

    let workflow_dispatch = on.get("workflow_dispatch");
    let repository_dispatch = on.get("repository_dispatch");

    if let Some(repository_dispatch) = repository_dispatch {
        if !repository_dispatch.is_null() {
            let types = repository_dispatch.get("types")
                .and_then(|t| t.as_array())
                .map(|v| {
                    v.iter()
                        .filter_map(|t| t.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            tracing::warn!("Ignoring repository_dispatch workflow: {} with types: {}", path.display(), types.join(","));
        }
    }

    if workflow_dispatch.is_none() {
        tracing::debug!("NONE; YAML={yaml}");
        tracing::debug!("ON: {:?}", on);
        return Ok(None);
    }
    let workflow_dispatch = workflow_dispatch.unwrap();

    let mut inputs = Vec::new();
    if let Some(inputs_hash) = workflow_dispatch.get("inputs").and_then(|i| i.as_object()) {
        for (name, v) in inputs_hash {
            let description = v.get("description").and_then(|s| s.as_str()).map(|s| s.to_string());
            let required = v.get("required").and_then(|s| s.as_bool()).unwrap_or(false);
            let default = v.get("default").and_then(|s| s.as_str()).map(|s| s.to_string());
            let ui_type = v.get("type").and_then(|s| s.as_str()).map(|s| s.to_string());
            let options = v.get("options")
                .and_then(|v| v.as_array())
                .map(|vec| {
                    vec.iter()
                        .filter_map(|opt| opt.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            // Logic from original code: required is true only if explicitly true AND no default
            let is_required = required && default.is_none();

            inputs.push(InputInfo {
                name: name.to_string(),
                description,
                required: is_required,
                default,
                ui_type,
                options,
            });
        }
    }

    let file = path.file_name().unwrap().to_string_lossy().to_string();
    let name = value.get("name").and_then(|s| s.as_str()).map(|s| s.to_string()).unwrap_or_else(|| file.clone());

    Ok(Some(WorkflowInfo { file, name, inputs }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_parse_workflow_dispatch_and_push() {
        let path = Path::new("tests/empty.yml");
        let result = parse_workflow(&path).unwrap();
        assert!(result.is_some(), "Workflow should be parsed when workflow_dispatch is present");
        let wf = result.unwrap();
        assert_eq!(wf.file, "empty.yml");
    }
}

// ... existing code ...

use serde::Serialize;

/// Rendering model (for template)
#[derive(Serialize)]
struct RenderModel {
    repo: String,
    reference: String,
    workflows: Vec<RenderWorkflow>,
    all_targets: Vec<String>,
}

#[derive(Serialize)]
struct RenderWorkflow {
    name: String,
    file: String,
    targets: Vec<RenderTarget>,
}

#[derive(Serialize)]
struct RenderTarget {
    target: String,
    comment_lines: Vec<String>,
    required_vars: Vec<String>,
    inputs_str: String,
}

/// Build the render model from parsed workflows and git defaults
fn build_render_model(base_dir: &Path, workflows: &[WorkflowInfo]) -> Result<RenderModel> {
    // Defaults from git
    let repo = git_utils::default_repo_from_git(base_dir)
        .map(|r| format!("{}/{}", r.owner, r.repo))
        .unwrap_or_else(|| "<owner>/<repo>".into());

    let reference = git_utils::default_ref_from_git(base_dir)
        .map(|r| r.to_string())
        .unwrap_or_else(|| "main".into());

    let mut render_workflows = Vec::new();
    let mut all_targets = Vec::new();

    for wf in workflows {
        // Join input names for info log
        let input_names = wf
            .inputs
            .iter()
            .map(|i| i.name.as_ref())
            .collect::<Vec<_>>()
            .join(", ");
        tracing::info!("workflow_dispatch: {}({input_names})", wf.file);

        let base_target = wf
            .file
            .trim_end_matches(".yml")
            .trim_end_matches(".yaml")
            .to_string();

        let mut targets = Vec::new();

        // If the first input is a choice → generate per option
        if let Some(first) = wf.inputs.first() {
            if first.ui_type.as_deref() == Some("choice") && !first.options.is_empty() {
                for opt in &first.options {
                    let tname = format!("{}-{}", base_target, opt.to_lowercase().replace(':', "_"));
                    targets.push(build_render_target(&tname, wf, Some((&first.name, opt))));
                }
            } else {
                targets.push(build_render_target(&base_target, wf, None));
            }
        } else {
            // Workflow without inputs
            targets.push(build_render_target(&base_target, wf, None));
        }

        all_targets.extend(targets.iter().map(|t| t.target.clone()));
        render_workflows.push(RenderWorkflow {
            name: wf.name.clone(),
            file: wf.file.clone(),
            targets,
        });
    }

    Ok(RenderModel {
        repo,
        reference,
        workflows: render_workflows,
        all_targets,
    })
}

fn build_render_target(
    target: &str,
    wf: &WorkflowInfo,
    choice: Option<(&String, &String)>,
) -> RenderTarget {
    // Header comment lines
    let mut comment_lines = Vec::new();
    comment_lines.push(format!("{} ({})", wf.name, wf.file));
    for inp in &wf.inputs {
        comment_lines.push(format!(
            "- {}:{}\t {}{}{}",
            inp.name.to_uppercase(),
            inp.ui_type.as_deref().unwrap_or("STRING"),
            inp.description.as_deref().unwrap_or(""),
            if inp.required { " (required)" } else { "" },
            inp.default
                .as_ref()
                .map(|d| format!(" [default: {}]", if d.len() < 256 {
                    d.to_string()
                } else {
                    format!("(long default: {} bytes)", d.len())
                }))
                .unwrap_or_default(),
        ));
    }

    // Required variables for checks
    let mut required_vars = Vec::new();
    for inp in &wf.inputs {
        if inp.required {
            required_vars.push(inp.name.to_uppercase());
        }
    }

    // Inputs JSON string
    let inputs_str = make_inputs(wf.inputs.as_slice(), choice);

    RenderTarget {
        target: target.to_string(),
        comment_lines,
        required_vars,
        inputs_str,
    }
}

/// Build only the `inputs` JSON fragment
fn make_inputs(inputs: &[InputInfo], choice: Option<(&String, &String)>) -> String {
    let mut parts = Vec::new();

    for inp in inputs {
        if let Some((cname, opt)) = &choice {
            if &inp.name == *cname {
                parts.push(format!("\"{}\":\"{}\"", inp.name, opt));
                continue;
            }
        }
        let var = inp.name.to_uppercase();
        parts.push(format!("\"{}\":\"$({})\"", inp.name, var));
    }

    parts.join("++|++")
}

/// Handlebars template for the Makefile
const MAKEFILE_TEMPLATE: &str = include_str!("template.Makefile");

/// Render model using the template (Handlebars)
fn render_with_template(model: &RenderModel) -> Result<String> {
    let mut handlebars = handlebars::Handlebars::new();
    // Makefile should not HTML-escape content
    handlebars.register_escape_fn(handlebars::no_escape);
    let out = handlebars
        .render_template(MAKEFILE_TEMPLATE, model)
        .context("failed to render Makefile template")?;
    Ok(out)
}