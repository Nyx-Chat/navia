fn main() {
    // Ensure 16KB alignment for Android builds
    let target = std::env::var("TARGET").unwrap_or_default();

    if target.contains("android") {
        // These flags are already set in .cargo/config.toml for Android targets
        // Adding them here would duplicate them

        // Additional alignment flags for NDK r27+
        if let Ok(ndk_home) = std::env::var("ANDROID_NDK_HOME") {
            if ndk_home.contains("/27.") || ndk_home.contains("/28.") {
                // NDK r27+ supports ANDROID_SUPPORT_FLEXIBLE_PAGE_SIZES
                println!("cargo:rustc-env=ANDROID_SUPPORT_FLEXIBLE_PAGE_SIZES=ON");
            }
        }
    }
}
