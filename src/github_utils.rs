use std::fs;
use std::path::Path;

pub fn default_workflow_from_dir() -> Option<String> {
    let workflows_dir = Path::new(".github/workflows");
    if !workflows_dir.exists() {
        return None;
    }

    let mut workflow_files = vec![];
    if let Ok(entries) = fs::read_dir(workflows_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext == "yml" || ext == "yaml" {
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        workflow_files.push(file_name.to_string());
                    }
                }
            }
        }
    }

    match workflow_files.len() {
        0 => None,
        1 => Some(workflow_files[0].clone()),
        _ => {
            eprintln!("Multiple workflows found in .github/workflows/:");
            for wf in &workflow_files {
                eprintln!("  {}", wf);
            }
            None
        }
    }
}

