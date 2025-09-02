use serde::Deserialize;
use std::{fs, path::Path, io::Write};
use anyhow::{Result, Context};

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

pub fn generate_makefile(output: &Path) -> Result<()> {
    let mut workflows = vec![];

    // Discover workflow files
    let dir = Path::new(".github/workflows");
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext == "yml" || ext == "yaml" {
                    workflows.push(path);
                }
            }
        }
    }

    let mut out = fs::File::create(output)
        .with_context(|| format!("failed to create output file {}", output.display()))?;

    writeln!(out, "GITHUB_TOKEN ?=")?;
    writeln!(out, "CURL_BASE = curl -sSL -H \"Accept: application/vnd.github+json\" \\\n\
                   \t-H \"Authorization: Bearer $$(GITHUB_TOKEN)\" \\\n\
                   \t-H \"X-GitHub-Api-Version: 2022-11-28\"")?;
    writeln!(out)?;

    for wf in workflows {
        let text = fs::read_to_string(&wf)?;
        let workflow: Workflow = serde_yaml::from_str(&text)
            .with_context(|| format!("failed to parse {}", wf.display()))?;

        if let Some(dispatch) = workflow.on.workflow_dispatch {
            let wf_name = wf.file_stem().unwrap().to_string_lossy().to_string();

            // Handle choice input expansion
            if let Some((first_name, first_input)) = dispatch.inputs.iter().next() {
                if first_input.r#type.as_deref() == Some("choice") {
                    if let Some(options) = &first_input.options {
                        for opt in options {
                            generate_target(&mut out, &wf_name, Some((first_name, opt)), &dispatch)?;
                        }
                        continue;
                    }
                }
            }

            // Fallback: generate one target
            generate_target(&mut out, &wf_name, None, &dispatch)?;
        }
    }

    Ok(())
}

fn generate_target(
    out: &mut fs::File,
    wf_name: &str,
    choice: Option<(&String, &String)>,
    dispatch: &WorkflowDispatch,
) -> Result<()> {
    let mut target_name = wf_name.to_string();
    if let Some((_, opt)) = choice {
        target_name.push('-');
        target_name.push_str(&opt.to_uppercase());
    }

    writeln!(out, "{}:", target_name)?;

    // Required input checks
    for (name, input) in &dispatch.inputs {
        let var = name.to_uppercase();
        if input.required {
            writeln!(out, "\t@test -n \"$({})\" || (echo \"{} is required\"; exit 1)", var, var)?;
        }
    }

    // Build JSON payload
    let mut payload = String::from("{\\\"ref\\\":\\\"$$(REF)\\\",\\\"inputs\\\":{");
    let mut first = true;
    for (name, _) in &dispatch.inputs {
        if !first {
            payload.push(',');
        }
        first = false;

        if let Some((choice_name, opt)) = &choice {
            if name == *choice_name {
                payload.push_str(&format!("\\\"{}\\\":\\\"{}\\\"", name, opt));
                continue;
            }
        }

        payload.push_str(&format!("\\\"{}\\\":\\\"$({})\\\"", name, name.to_uppercase()));
    }
    payload.push_str("}}");

    writeln!(
        out,
        "\t$$(CURL_BASE) -X POST \\\n\
         \t  https://api.github.com/repos/$$(REPO)/actions/workflows/{}/dispatches \\\n\
         \t  -d \"{}\"",
        wf_name, payload
    )?;

    writeln!(out)?;
    Ok(())
}
