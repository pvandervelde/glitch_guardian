//! Copy the files for the azure function into a single directory
//!
//! For the azure function we need the following directories and files in the following layout.
//! Where [D] means directory and [F] means file
//!
//! [D] <function_name>       --> <target>/<function_name>
//!     [F] function.json     --> <target>/<function_name>/function.json
//! [F] host.json             --> <target>/host.json
//! [F] local.settings.json   --> <target>/local.settings.json
//! [F] handler               --> <target>/handler
//!

use anyhow::{anyhow, Context, Result};
use handlebars::Handlebars;
use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    path::PathBuf,
};

#[cfg(test)]
#[path = "main_tests.rs"]
mod main_tests;

fn copy_file(from: &PathBuf, to: &PathBuf) -> Result<()> {
    let target_dir = to.parent().ok_or_else(|| {
        anyhow!(
            "The target path is not a valid directory: {}",
            to.to_str().unwrap()
        )
    })?;
    fs::create_dir_all(target_dir).with_context(|| {
        format!(
            "Failed to create the target directory: {}",
            target_dir.to_str().unwrap()
        )
    })?;

    let file_size = fs::metadata(from.clone())
        .with_context(|| format!("Failed to get the file size of {}", from.to_str().unwrap()))?
        .len();
    let bytes_copied = fs::copy(from, to)?;

    if file_size != bytes_copied {
        // Failed to copy the file completely
        return Err(anyhow!(
            "Failed to copy {} to {}. Copied {} bytes but expected {} bytes",
            from.to_str().unwrap(),
            to.to_str().unwrap(),
            bytes_copied,
            file_size
        ));
    }

    Ok(())
}

// Creates the host file in the target location.
fn create_host_file(workspace_dir: &str, target_dir: &str, handler_file_name: &str) -> Result<()> {
    let host_json_template: PathBuf = [workspace_dir, "az_func", "host.json"].iter().collect();

    let host_json_target: PathBuf = [target_dir, "host.json"].iter().collect();

    let mut data = HashMap::new();
    data.insert("function_exe_path", handler_file_name);

    let mut handlebars = Handlebars::new();
    handlebars.set_strict_mode(true);
    handlebars
        .register_template_file("host_json", host_json_template.as_path())
        .with_context(|| {
            format!(
                "Failed to read the host.json template file at {}",
                host_json_template.to_str().unwrap()
            )
        })?;

    let mut output_file = File::create(host_json_target.as_path()).with_context(|| {
        format!(
            "Failed to create the host.json file at {}",
            host_json_target.to_str().unwrap()
        )
    })?;
    handlebars
        .render_to_write("host_json", &data, &mut output_file)
        .with_context(|| {
            format!(
                "Failed to set the content of the host.json file at {}",
                host_json_target.to_str().unwrap()
            )
        })?;

    Ok(())
}

/// Get the path to the workspace root.
///
/// Assumed this xtask is located in `[WORKSPACE]/lib/xtask`.
fn path_to_workspace_root() -> Result<PathBuf> {
    let pkg_root = std::env!("CARGO_MANIFEST_DIR");
    let pkg_path = PathBuf::from(pkg_root);
    let mut result = pkg_path.clone();

    // Get the lib path
    if !result.pop() {
        return Err(anyhow!(
            "Failed to determine the workspace root by getting the first parent of: {}",
            pkg_path.to_str().unwrap()
        ));
    }

    if !result.pop() {
        return Err(anyhow!(
            "Failed to determine the workspace root by getting the second parent of: {}",
            pkg_path.to_str().unwrap()
        ));
    }

    Ok(result)
}

fn main() -> anyhow::Result<()> {
    let function_name = "glitch_guardian";

    let workspace_root_path = path_to_workspace_root()?;
    let workspace_root_str = workspace_root_path.to_str().unwrap();

    let target_path: PathBuf = [workspace_root_str, "target", "func"].iter().collect();
    let target_path_str = target_path.to_str().unwrap();

    // For the azure function we need the following directories and files in the following layout.
    // Where [D] means directory and [F] means file
    //
    // [D] <function_name>       --> <target>/<function_name>
    //     [F] function.json     --> <target>/<function_name>/function.json
    // [F] host.json             --> <target>/host.json
    // [F] local.settings.json   --> <target>/local.settings.json
    // [F] handler               --> <target>/handler

    let handler_name = match env::consts::OS {
        "linux" => "handler",
        "windows" => "handler.exe",
        _ => "handler",
    };

    let handler_src = match env::consts::OS {
        "linux" => [
            workspace_root_str,
            "target",
            "x86_64-unknown-linux-musl",
            "release",
            handler_name,
        ]
        .iter()
        .collect(),
        "windows" => [workspace_root_str, "target", "release", handler_name]
            .iter()
            .collect(),
        _ => [workspace_root_str, "target", "release", handler_name]
            .iter()
            .collect(),
    };

    // function.json
    let function_json_src: PathBuf = [
        workspace_root_str,
        "az_func",
        function_name,
        "function.json",
    ]
    .iter()
    .collect();
    let function_json_target: PathBuf = [target_path_str, function_name, "function.json"]
        .iter()
        .collect();
    copy_file(&function_json_src, &function_json_target)?;

    // local.settings.json
    let local_setting_json_src = [workspace_root_str, "az_func", "local.settings.json"]
        .iter()
        .collect();
    let local_setting_json_target = [target_path_str, "local.settings.json"].iter().collect();
    copy_file(&local_setting_json_src, &local_setting_json_target)?;

    let handler_target = [target_path_str, handler_name].iter().collect();
    copy_file(&handler_src, &handler_target)?;

    create_host_file(workspace_root_str, target_path_str, handler_name)?;

    Ok(())
}
