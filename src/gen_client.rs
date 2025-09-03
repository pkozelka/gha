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
    let content = render_makefile(&dir, &workflows)?;
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

    Ok(infos)
}

/// Parse a workflow into WorkflowInfo
fn parse_workflow(path: &Path) -> Result<Option<WorkflowInfo>> {
    let text = fs::read_to_string(path)?;
    let wf: WorkflowRaw = serde_yaml::from_str(&text)
        .with_context(|| format!("failed to parse {}", path.display()))?;

    if let Some(d) = wf.on.repository_dispatch {
        tracing::debug!("Ignoring repository_dispatch workflow: {} with types: {}", path.display(), d.types.join(","));
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

/// Render full Makefile text from collected workflows
fn render_makefile(base_dir: &Path, workflows: &[WorkflowInfo]) -> Result<String> {
    let mut buf = String::new();

    // Defaults from git
    let repo = git_utils::default_repo_from_git(base_dir)
        .map(|r| format!("{}/{}", r.owner, r.repo))
        .unwrap_or_else(|| "<owner>/<repo>".into());
    let reference = git_utils::default_ref_from_git(base_dir)
        .map(|r| r.to_string())
        .unwrap_or_else(|| "main".into());

    // Shared vars
    buf.push_str(&format!("REPO ?= {repo}\n"));
    buf.push_str(&format!("REF ?= {reference}\n\n"));
    buf.push_str("# Authentication\n");
    buf.push_str("GITHUB_TOKEN ?=\n");
    buf.push_str("CURL_AUTH ?=-H \"Authorization: Bearer $(GITHUB_TOKEN)\"\n");
    buf.push_str("# or, in case you prefer ~/.netrc:\n");
    buf.push_str("#CURL_AUTH=--netrc\n");


    // Macro: wraps the JSON envelope
    buf.push_str("\ndefine WORKFLOW_DISPATCH\n");
    buf.push_str("\tcurl -vSL 'https://api.github.com/repos/$(REPO)/actions/workflows/$1/dispatches' \\\n");
    buf.push_str("\t$(CURL_AUTH) \\\n");
    buf.push_str("\t-H \"X-GitHub-Api-Version: 2022-11-28\" \\\n");
    buf.push_str("\t-H \"Accept: application/vnd.github+json\" \\\n");
    buf.push_str("\t-d '{\"ref\":\"$(REF)\",\"inputs\":{$2}}'\n");
    buf.push_str("endef\n\n");

    let mut all_targets = Vec::new();
    for wf in workflows {
        let targets = render_workflow(&mut buf, wf)?;
        all_targets.extend(targets);
    }

    buf.push_str(".PHONY: ");
    for t in &all_targets {
        buf.push_str(t);
        buf.push(' ');
    }
    buf.push('\n');

    Ok(buf)
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

    parts.join(",")
}
/// Render a single workflow block, returning all target names
fn render_workflow(buf: &mut String, wf: &WorkflowInfo) -> Result<Vec<String>> {
    // join input names into one string, comma separated
    let input_names = wf.inputs.iter().map(|i| i.name.as_ref()).collect::<Vec<_>>().join(", ");
    tracing::info!("workflow_dispatch: {}({input_names})", wf.file );
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
                let tname = format!("{}-{}", base_target, opt.to_lowercase().replace(':',"_"));
                render_target(buf, &tname, wf, Some((&first.name, opt)))?;
                targets.push(tname);
            }
            return Ok(targets);
        }
    }

    // Default: single target
    render_target(buf, &base_target, wf, None)?;
    targets.push(base_target);
    Ok(targets)
}

/// Render one target
fn render_target(
    buf: &mut String,
    target: &str,
    wf: &WorkflowInfo,
    choice: Option<(&String, &String)>,
) -> Result<()> {
    // Header
    buf.push_str(&format!("##\n# {} ({})\n", wf.name, wf.file));
    for inp in &wf.inputs {
        buf.push_str(&format!(
            "# - {}:{}\t {}{}{}\n",
            inp.name.to_uppercase(),
            inp.ui_type.as_deref().unwrap_or("STRING"),
            inp.description.as_deref().unwrap_or(""),
            if inp.required { " (required)" } else { "" },
            inp.default
                .as_ref()
                .map(|d| format!(" [default: {}]", d))
                .unwrap_or_default(),
        ));
    }

    buf.push_str(&format!("{target}:\n"));

    // Required checks inline with comment
    for inp in &wf.inputs {
        if inp.required {
            let var = inp.name.to_uppercase();
            buf.push_str(&format!("\ttest -n \"$({var})\" # requires: {var}\n"));
        }
    }

    // Payload inputs only
    let inputs_str = make_inputs(wf.inputs.as_slice(), choice);
    buf.push_str(&format!("\t$(call WORKFLOW_DISPATCH,{},{})\n\n", wf.file, inputs_str));

    Ok(())
}
