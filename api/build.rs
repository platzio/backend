use std::process::Command;

fn main() {
    let output = Command::new("git")
        .args(["describe", "--tags", "--always"])
        .output()
        .expect("failed to execute process");

    if !output.status.success() {
        panic!("git-describe failed!");
    }
    let version = String::from_utf8_lossy(&output.stdout).into_owned();

    println!("cargo:rustc-env=PLATZ_BACKEND_VERSION={version}");
}
