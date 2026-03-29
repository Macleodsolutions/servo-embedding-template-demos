use std::path::PathBuf;
use std::process::Command;

fn main() {
    let ui_dir = PathBuf::from("ui");

    if ui_dir.join("package.json").exists() {
        let yarn = find_yarn().expect("Could not find yarn — ensure it is installed and in PATH");

        let status = Command::new(&yarn)
            .arg("build")
            .current_dir(&ui_dir)
            .status()
            .expect("Failed to run yarn build");

        if !status.success() {
            panic!("yarn build failed");
        }

        if !ui_dir.join("dist").exists() {
            panic!("yarn build succeeded but dist/ was not created");
        }
    }

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let ui_dist = PathBuf::from(&manifest_dir).join("ui").join("dist");
    println!("cargo:rustc-env=SERVO_UI_DIST={}", ui_dist.display());

    println!("cargo:rerun-if-changed=ui/src");
    println!("cargo:rerun-if-changed=ui/package.json");
    println!("cargo:rerun-if-changed=ui/vite.config.ts");
}

fn find_yarn() -> Option<PathBuf> {
    if Command::new("yarn").arg("--version").output().is_ok() {
        return Some(PathBuf::from("yarn"));
    }

    let path_var = std::env::var("PATH").unwrap_or_default();
    let candidates = if cfg!(windows) {
        vec!["yarn.cmd", "yarn.ps1", "yarn"]
    } else {
        vec!["yarn"]
    };

    for dir in std::env::split_paths(&path_var) {
        for candidate in &candidates {
            let full = dir.join(candidate);
            if full.exists() {
                return Some(full);
            }
        }
    }

    #[cfg(windows)]
    if let Ok(appdata) = std::env::var("APPDATA") {
        let candidate = PathBuf::from(appdata).join("npm").join("yarn.cmd");
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}
