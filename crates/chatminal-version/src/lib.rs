pub fn engine_version() -> &'static str {
    // See build.rs
    env!("ENGINE_CI_TAG")
}

pub fn engine_target_triple() -> &'static str {
    // See build.rs
    env!("ENGINE_TARGET_TRIPLE")
}
