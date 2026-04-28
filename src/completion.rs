use clap::ValueEnum;
use clap_complete::generate;
use std::io;
use std::io::Write;
use std::path::Path;

use crate::gen_client::parse_workflow;

/// Shell type for completion generation
#[derive(ValueEnum, Clone, Debug)]
pub enum Shell {
    /// Bash shell
    Bash,
    /// Zsh shell
    Zsh,
    /// Fish shell
    Fish,
    /// Powershell
    Powershell,
    /// Elvish shell
    Elvish,
}

impl From<Shell> for clap_complete::shells::Shell {
    fn from(shell: Shell) -> Self {
        match shell {
            Shell::Bash => clap_complete::shells::Shell::Bash,
            Shell::Zsh => clap_complete::shells::Shell::Zsh,
            Shell::Fish => clap_complete::shells::Shell::Fish,
            Shell::Powershell => clap_complete::shells::Shell::PowerShell,
            Shell::Elvish => clap_complete::shells::Shell::Elvish,
        }
    }
}

/// Generate shell completions, with extra dynamic completion helpers for workflow inputs.
///
/// For Zsh and Bash, appends an additional function that completes `key=` argument values
/// for `choice`-type workflow inputs found in `.github/workflows/`.
pub fn generate_completion(shell: Shell) -> io::Result<()> {
    use clap::CommandFactory;
    let mut cmd = crate::Cli::command();
    let clap_shell: clap_complete::shells::Shell = shell.clone().into();
    let mut out = io::stdout();
    generate(clap_shell, &mut cmd, "gha", &mut out);

    // Emit dynamic-completion helper functions for shells that support them well
    match shell {
        Shell::Zsh => {
            let helpers = build_zsh_dynamic_helpers(Path::new(".github/workflows"));
            out.write_all(helpers.as_bytes())?;
        }
        Shell::Bash => {
            let helpers = build_bash_dynamic_helpers(Path::new(".github/workflows"));
            out.write_all(helpers.as_bytes())?;
        }
        _ => {}
    }

    Ok(())
}

/// Build a Zsh snippet that completes `name=<value>` for choice-type workflow inputs.
///
/// For each workflow found in `workflows_dir` that has `choice` inputs, we create
/// a mapping so zsh can suggest the valid option values when the user types e.g. `env=`.
fn build_zsh_dynamic_helpers(workflows_dir: &Path) -> String {
    let choices = collect_choice_inputs(workflows_dir);
    if choices.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    out.push_str("\n# --- gha dynamic workflow input completions ---\n");
    out.push_str("_gha_run_args() {\n");
    out.push_str("  local -A _gha_choices\n");
    for (input_name, options) in &choices {
        let values = options
            .iter()
            .map(|o| format!("'{}'", o.replace('\'', "\\'")))
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str(&format!(
            "  _gha_choices[{input_name}]=({values})\n"
        ));
    }
    out.push_str(
        r#"  local cur="$words[$CURRENT]"
  local key="${cur%%=*}"
  if [[ "$cur" == *=* ]] && (( ${+_gha_choices[$key]} )); then
    local prefix="${cur%%=*}="
    compadd -P "$prefix" -- ${=_gha_choices[$key]}
  else
    # complete keys
    local keys=(${(k)_gha_choices})
    compadd -S '=' -- $keys
  fi
}
# Override the generated _gha__run function's argument completion
(( $+functions[_gha__run] )) && {
  local _orig_gha_run=$(typeset -f _gha__run)
  eval "${_orig_gha_run/# *state=ARG*/_gha_run_args; return}"
}
"#,
    );
    out
}

/// Build a Bash snippet that completes `name=<value>` for choice-type workflow inputs.
fn build_bash_dynamic_helpers(workflows_dir: &Path) -> String {
    let choices = collect_choice_inputs(workflows_dir);
    if choices.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    out.push_str("\n# --- gha dynamic workflow input completions ---\n");
    out.push_str("_gha_run_args_complete() {\n");
    out.push_str("  local cur=\"${COMP_WORDS[COMP_CWORD]}\"\n");
    out.push_str("  local key=\"${cur%%=*}\"\n");
    out.push_str("  case \"$key\" in\n");
    for (input_name, options) in &choices {
        let values = options
            .iter()
            .map(|o| format!("\"{}\"", o.replace('"', "\\\"")))
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str(&format!(
            "    {input_name}) COMPREPLY=( $(compgen -W {values} -- \"${{cur#*=}}\") ); return ;;\n"
        ));
    }
    out.push_str("  esac\n");
    // fallback: complete known keys
    let all_keys = choices.keys().cloned().collect::<Vec<_>>().join(" ");
    out.push_str(&format!(
        "  COMPREPLY=( $(compgen -W \"{all_keys}\" -- \"$cur\") )\n"
    ));
    out.push_str("}\n");
    // Patch the generated _gha function to call our helper for trailing args
    out.push_str(
        r#"# Augment generated completion to use dynamic input helpers for 'run' ARG positions
_gha_orig_complete="${_gha}"
complete -F _gha gha
"#,
    );
    out
}

/// Collect all (input_name -> [options]) for `choice`-typed inputs from all workflows in dir.
fn collect_choice_inputs(workflows_dir: &Path) -> std::collections::BTreeMap<String, Vec<String>> {
    let mut result = std::collections::BTreeMap::new();
    let Ok(entries) = std::fs::read_dir(workflows_dir) else {
        return result;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "yml" && ext != "yaml" {
            continue;
        }
        let Ok(Some(info)) = parse_workflow(&path) else {
            continue;
        };
        for input in info.inputs {
            if input.ui_type.as_deref() == Some("choice") && !input.options.is_empty() {
                result.entry(input.name).or_insert(input.options);
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zsh_helper_empty_when_no_workflows() {
        let result = build_zsh_dynamic_helpers(Path::new("/nonexistent"));
        assert!(result.is_empty());
    }

    #[test]
    fn bash_helper_empty_when_no_workflows() {
        let result = build_bash_dynamic_helpers(Path::new("/nonexistent"));
        assert!(result.is_empty());
    }

    #[test]
    fn collect_choice_inputs_returns_empty_for_missing_dir() {
        let choices = collect_choice_inputs(Path::new("/nonexistent"));
        assert!(choices.is_empty());
    }

    #[test]
    fn collect_choice_inputs_finds_options_from_fixture() {
        let choices = collect_choice_inputs(Path::new("tests"));
        let env_opts = choices.get("environment").expect("should find 'environment' choice input");
        assert!(env_opts.contains(&"dev".to_string()));
        assert!(env_opts.contains(&"staging".to_string()));
        assert!(env_opts.contains(&"prod".to_string()));
    }

    #[test]
    fn zsh_helper_contains_choice_values() {
        let result = build_zsh_dynamic_helpers(Path::new("tests"));
        assert!(result.contains("_gha_run_args"), "should define _gha_run_args function");
        assert!(result.contains("environment"), "should include choice input name");
        assert!(result.contains("'dev'"), "should include choice option value");
        assert!(result.contains("'staging'"), "should include choice option value");
        assert!(result.contains("'prod'"), "should include choice option value");
    }

    #[test]
    fn bash_helper_contains_choice_values() {
        let result = build_bash_dynamic_helpers(Path::new("tests"));
        assert!(result.contains("_gha_run_args_complete"), "should define completion function");
        assert!(result.contains("environment"), "should include choice input name");
        assert!(result.contains("\"dev\""), "should include choice option value");
        assert!(result.contains("\"staging\""), "should include choice option value");
        assert!(result.contains("\"prod\""), "should include choice option value");
    }

    #[test]
    fn string_type_inputs_not_in_choice_completions() {
        let choices = collect_choice_inputs(Path::new("tests"));
        // "version" is type: string, not choice
        assert!(!choices.contains_key("version"), "string inputs should not appear in choice completions");
    }
}





