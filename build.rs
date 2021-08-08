use std::process::Command;

fn main() {
    let output = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let git_hash = String::from_utf8(output.stdout).unwrap();

    if git_hash.len() > 0 {
        println!("cargo:rustc-env=GIT_HASH={}", git_hash);
        println!(
            "cargo:rustc-env=FULL_VERSION=v{} @ {}",
            env!("CARGO_PKG_VERSION"),
            git_hash
        );
    }
}
