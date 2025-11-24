use crate::git_utils;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::{
    fs,
    path::Path,
};

#[derive(Debug, Deserialize)]
struct WorkflowRaw {
    name: Option<String>,
    #[serde(default)]
    on: WorkflowOn,
}

#[derive(Debug, Deserialize, Default)]
struct WorkflowOn {
    #[serde(rename = "workflow_dispatch", default)]
    workflow_dispatch: Option<WorkflowDispatch>,
    #[serde(rename = "repository_dispatch", default)]
    repository_dispatch: Option<RepositoryDispatch>,
}

#[derive(Debug, Deserialize)]
struct RepositoryDispatch {
    #[serde(default)]
    types: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct WorkflowDispatch {
    #[serde(default, deserialize_with = "deserialize_ordered_inputs")]
    inputs: Vec<(String, WorkflowInput)>,
}

#[derive(Debug, Deserialize)]
struct WorkflowInput {
    description: Option<String>,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    default: Option<String>,
    #[serde(rename = "type", default)]
    r#type: Option<String>, // e.g. "choice"
    #[serde(default)]
    options: Option<Vec<String>>,
}

// Custom deserializer to preserve mapping order for `inputs`
fn deserialize_ordered_inputs<'de, D>(
    deserializer: D,
) -> Result<Vec<(String, WorkflowInput)>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct OrderedVisitor;

    impl<'de> serde::de::Visitor<'de> for OrderedVisitor {
        type Value = Vec<(String, WorkflowInput)>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a mapping of inputs preserving order")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut vec = Vec::new();
            while let Some((k, v)) = map.next_entry::<String, WorkflowInput>()? {
                vec.push((k, v));
            }
            Ok(vec)
        }
    }

    deserializer.deserialize_map(OrderedVisitor)
}

/// Normalized workflow info
struct WorkflowInfo {
    pub file: String,
    pub name: String,
    inputs: Vec<InputInfo>,
}

struct InputInfo {
    name: String,
    description: Option<String>,
    required: bool,
    default: Option<String>,
    ui_type: Option<String>,
    options: Vec<String>,
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
fn parse_workflow(path: &Path) -> Result<Option<WorkflowInfo>> {
    let text = fs::read_to_string(path)?;
    let wf: WorkflowRaw = serde_yaml::from_str(&text)
        .with_context(|| format!("failed to parse {}", path.display()))?;

    if let Some(d) = wf.on.repository_dispatch {
        // TODO maybe let's generate client, too - repository_dispatch-EVENTNAME with CLIENT_PAYLOAD input
        tracing::warn!("Ignoring repository_dispatch workflow: {} with types: {}", path.display(), d.types.join(","));
    }
    let dispatch = match wf.on.workflow_dispatch {
        Some(d) => d,
        None => return Ok(None),
    };

    let inputs = dispatch
        .inputs
        .into_iter()
        .map(|(name, raw)| InputInfo {
            name,
            description: raw.description,
            required: raw.required && raw.default.is_none(),
            default: raw.default,
            ui_type: raw.r#type,
            options: raw.options.unwrap_or_default(),
        })
        .collect();

    let file = path.file_name().unwrap().to_string_lossy().to_string();
    let name = wf.name.unwrap_or_else(|| file.clone());

    Ok(Some(WorkflowInfo { file, name, inputs }))
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