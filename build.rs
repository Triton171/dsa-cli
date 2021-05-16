fn main() {
    let mut git_hash = match std::fs::read_to_string(".git/HEAD") {
        Ok(s) => s,
        _ => String::from("unknown"),
    };
    if git_hash.chars().count() > 7 {
        git_hash = String::from(&git_hash[..7]); // Short hash; "unknown" is conviniently 7 chars long
    }
    println!("cargo:rustc-rerun-if-changed=.git/HEAD");

    if git_hash.len() > 0 {
        println!("cargo:rustc-env=GIT_HASH={}", git_hash);
        println!(
            "cargo:rustc-env=FULL_VERSION=v{} @ {}",
            env!("CARGO_PKG_VERSION"),
            git_hash
        );
    }
}
