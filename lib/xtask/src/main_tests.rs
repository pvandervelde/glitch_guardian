use super::*;
use std::io::Write;
use tempfile::TempDir;

fn create_temp_file(content: &str) -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("temp_file.txt");
    let mut file = File::create(&file_path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
    (dir, file_path)
}

#[test]
fn when_copying_existing_file_it_should_succeed() {
    let (source_dir, source_file) = create_temp_file("test content");
    let target_dir = TempDir::new().unwrap();
    let target_file = target_dir.path().join("target_file.txt");

    copy_file(&source_file, &target_file).unwrap();

    assert!(target_file.exists());
    assert_eq!(
        fs::read_to_string(&source_file).unwrap(),
        fs::read_to_string(&target_file).unwrap()
    );
}

#[test]
fn when_copying_non_existing_file_it_should_error() {
    let source_file = PathBuf::from("/nonexistent/file.txt");
    let target_dir = TempDir::new().unwrap();
    let target_file = target_dir.path().join("target_file.txt");

    assert!(copy_file(&source_file, &target_file).is_err());
}

#[test]
fn when_creating_host_file_it_should_provide_a_complete_file() {
    let workspace_dir = TempDir::new().unwrap();
    let target_dir = TempDir::new().unwrap();

    // Create a mock host.json template
    let az_func_dir = workspace_dir.path().join("az_func");
    fs::create_dir_all(&az_func_dir).unwrap();
    let host_json_template = az_func_dir.join("host.json");
    fs::write(&host_json_template, r#"{"customHandler": {"description": {"defaultExecutablePath": "{{function_exe_path}}"}}}}"#).unwrap();

    create_host_file(
        workspace_dir.path().to_str().unwrap(),
        target_dir.path().to_str().unwrap(),
        "test_handler",
    )
    .unwrap();

    let created_host_json = target_dir.path().join("host.json");
    assert!(created_host_json.exists());

    let content = fs::read_to_string(created_host_json).unwrap();
    assert!(content.contains("test_handler"));
}

#[test]
fn test_path_to_workspace_root() {
    let result = path_to_workspace_root();
    assert!(result.is_ok());
    let workspace_root = result.unwrap();
    //assert!(workspace_root.ends_with("workspace_name")); // Replace with actual workspace name
    assert!(workspace_root.join("lib").join("xtask").exists());
}
