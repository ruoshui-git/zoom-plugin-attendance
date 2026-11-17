use serde_json::Value;
use std::{fs, path::Path};

fn walk(path: &Path, files: &mut Vec<std::path::PathBuf>) {
    for item in fs::read_dir(path).unwrap() {
        let path = item.unwrap().path();
        if path.is_dir() {
            walk(&path, files);
        } else if path.extension().and_then(|x| x.to_str()) == Some("json") {
            files.push(path);
        }
    }
}

#[test]
fn reference_fixture_inputs_are_valid_and_adaptable() {
    let root = Path::new("tests/parity/fixtures");
    let mut files = Vec::new();
    walk(root, &mut files);
    files.sort();
    assert_eq!(
        files.len(),
        56,
        "fixture inventory changed; update parity metadata"
    );
    for path in files {
        let input: Value = serde_json::from_slice(&fs::read(&path).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let result = attend_engine::adapt_zoom_apps_events(&input);
        assert!(result.is_ok(), "{}: {:?}", path.display(), result.err());
    }
}

#[test]
fn pinyin_matches_tiny_pinyin_for_fixture_and_config_names() {
    #[derive(serde::Deserialize)]
    struct Golden {
        name: String,
        expected: String,
    }
    let data: Vec<Golden> =
        serde_json::from_slice(&fs::read("tests/parity/pinyin-goldens.json").unwrap()).unwrap();
    let mut seen = 0;
    for item in data.iter().filter(|item| item.name.chars().count() > 1) {
        seen += 1;
        assert_eq!(
            attend_engine::build_pinyin(&item.name),
            item.expected,
            "pinyin mismatch for {:?}",
            item.name
        );
    }
    assert!(seen >= 200, "pinyin corpus unexpectedly shrank: {seen}");
}
