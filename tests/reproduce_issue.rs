#[cfg(test)]
mod tests {
    #[test]
    fn test_serde_yml_null() {
        let yaml = "workflow_dispatch:";
        let value: serde_json::Value = serde_yml::from_str(yaml).unwrap();
        println!("Parsed serde_yml: {:?}", value);
        assert!(value.get("workflow_dispatch").is_some());
        assert!(value.get("workflow_dispatch").unwrap().is_null());
    }

    #[test]
    fn test_serde_yml_explicit_null() {
        let yaml = "workflow_dispatch: null";
        let value: serde_json::Value = serde_yml::from_str(yaml).unwrap();
        println!("Parsed serde_yml explicit: {:?}", value);
        assert!(value.get("workflow_dispatch").unwrap().is_null());
    }

    #[test]
    fn test_serde_yml_syntax() {
        let yaml = r#"
jobs:
  build-images:
    if: ${{ github.event_name == 'schedule' || github.event_name == 'push' || (github.event_name == 'workflow_dispatch' && github.event.inputs.rebuild_docker == 'true') }}
    strategy:
      matrix:
        runner: [
          "turbo-prague-small",
          "turbo-prague",
          "tp2",
        ]
"#;
        let value: serde_json::Value = serde_yml::from_str(yaml).expect("serde_yml should parse multi-line array");
        assert!(value["jobs"]["build-images"]["strategy"]["matrix"]["runner"].is_array());
    }
}
