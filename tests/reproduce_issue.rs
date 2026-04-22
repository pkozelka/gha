use yaml_rust2::YamlLoader;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_dispatch_empty() {
        let yaml = "workflow_dispatch:";
        let docs = YamlLoader::load_from_str(yaml).unwrap();
        let doc = &docs[0];
        let workflow_dispatch = &doc["workflow_dispatch"];
        println!("Parsed: {:?}", workflow_dispatch);
        assert!(!workflow_dispatch.is_badvalue(), "workflow_dispatch should not be BadValue even if empty in YAML");
        assert!(workflow_dispatch.is_null());
    }

    #[test]
    fn test_workflow_dispatch_null() {
        let yaml = "workflow_dispatch: null";
        let docs = YamlLoader::load_from_str(yaml).unwrap();
        let doc = &docs[0];
        let workflow_dispatch = &doc["workflow_dispatch"];
        println!("Parsed: {:?}", workflow_dispatch);
        assert!(!workflow_dispatch.is_badvalue(), "workflow_dispatch should not be BadValue if it is explicitly null");
        assert!(workflow_dispatch.is_null());
    }

    #[test]
    fn test_workflow_dispatch_with_content() {
        let yaml = "workflow_dispatch:\n  inputs:\n    foo: bar";
        let docs = YamlLoader::load_from_str(yaml).unwrap();
        let doc = &docs[0];
        let workflow_dispatch = &doc["workflow_dispatch"];
        println!("Parsed: {:?}", workflow_dispatch);
        assert!(!workflow_dispatch.is_badvalue());
        assert!(!workflow_dispatch["inputs"].is_badvalue());
    }

    #[test]
    fn test_workflow_dispatch_missing() {
        let yaml = "other: value";
        let docs = YamlLoader::load_from_str(yaml).unwrap();
        let doc = &docs[0];
        let workflow_dispatch = &doc["workflow_dispatch"];
        println!("Parsed: {:?}", workflow_dispatch);
        assert!(workflow_dispatch.is_badvalue());
    }
}
