use crate::build;

/// Returns the long version string with build information
pub fn long_version() -> String {
    let version = build::PKG_VERSION;
    let git_sha = build::COMMIT_HASH;
    let git_branch = build::BRANCH;
    let git_dirty = if build::GIT_CLEAN { "clean" } else { "dirty" };

    let build_time = build::BUILD_TIME;
    let rust_version = build::RUST_VERSION;
    let build_os = build::BUILD_OS;
    let build_target = build::BUILD_TARGET;

    let commit_msg = option_env!("GIT_COMMIT_MESSAGE").unwrap_or("no commit message");

    format!(
        "{version} built from branch {git_branch} at commit {git_sha} {git_dirty} ({commit_msg})
Build: {build_time}
Target: {build_target} ({build_os})
Rustc: {rust_version}"
    )
}
