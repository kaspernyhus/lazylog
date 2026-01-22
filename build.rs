use shadow_rs::{BuildPattern, ShadowBuilder};
use std::process::Command;

fn main() -> shadow_rs::SdResult<()> {
    ShadowBuilder::builder().build_pattern(BuildPattern::RealTime).build()?;

    // Capture commit message at build time
    if let Ok(output) = Command::new("git").args(["log", "-1", "--pretty=%s"]).output()
        && let Ok(commit_msg) = String::from_utf8(output.stdout)
    {
        println!("cargo:rustc-env=GIT_COMMIT_MESSAGE={}", commit_msg.trim());
    }

    // Capture GitHub ref name for CI builds (tag or branch name)
    if let Ok(ref_name) = std::env::var("GITHUB_REF_NAME") {
        println!("cargo:rustc-env=GITHUB_REF_NAME={}", ref_name);
    }

    Ok(())
}
