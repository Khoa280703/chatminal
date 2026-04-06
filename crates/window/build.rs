#[cfg(all(unix, not(target_os = "macos")))]
use std::fs;
#[cfg(all(unix, not(target_os = "macos")))]
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    use gl_generator::{Api, Fallbacks, Profile, Registry};
    use std::env;
    use std::fs::File;
    use std::path::PathBuf;

    configure_linux_link_shims();

    let dest = PathBuf::from(&env::var("OUT_DIR").unwrap());
    let target = env::var("TARGET").unwrap();
    let mut file = File::create(dest.join("egl_bindings.rs")).unwrap();
    let reg = Registry::new(
        Api::Egl,
        (1, 5),
        Profile::Core,
        Fallbacks::All,
        [
            "EGL_KHR_create_context",
            "EGL_EXT_create_context_robustness",
            "EGL_KHR_create_context_no_error",
            "EGL_KHR_platform_x11",
            "EGL_KHR_platform_android",
            "EGL_KHR_platform_wayland",
            "EGL_KHR_platform_gbm",
            "EGL_EXT_platform_base",
            "EGL_EXT_platform_x11",
            "EGL_MESA_platform_gbm",
            "EGL_EXT_platform_wayland",
            "EGL_EXT_platform_device",
            "EGL_KHR_swap_buffers_with_damage",
        ],
    );

    if target.contains("android") || target.contains("ios") {
        reg.write_bindings(gl_generator::StaticStructGenerator, &mut file)
    } else {
        reg.write_bindings(gl_generator::StructGenerator, &mut file)
    }
    .unwrap();

    if target.contains("apple") {
        println!("cargo:rustc-link-lib=framework=Carbon");
    }

    if target.contains("windows") {
        let mut file = File::create(dest.join("wgl_bindings.rs")).unwrap();
        let reg = Registry::new(Api::Wgl, (1, 0), Profile::Core, Fallbacks::All, []);

        reg.write_bindings(gl_generator::StructGenerator, &mut file)
            .unwrap();

        let mut file = File::create(dest.join("wgl_extra_bindings.rs")).unwrap();
        Registry::new(
            Api::Wgl,
            (1, 0),
            Profile::Core,
            Fallbacks::All,
            [
                "WGL_ARB_create_context",
                "WGL_ARB_create_context_profile",
                "WGL_ARB_create_context_robustness",
                "WGL_ARB_context_flush_control",
                "WGL_ARB_extensions_string",
                "WGL_ARB_framebuffer_sRGB",
                "WGL_ARB_multisample",
                "WGL_ARB_pixel_format",
                "WGL_ARB_pixel_format_float",
                "WGL_EXT_create_context_es2_profile",
                "WGL_EXT_extensions_string",
                "WGL_EXT_framebuffer_sRGB",
                "WGL_EXT_swap_control",
            ],
        )
        .write_bindings(gl_generator::StructGenerator, &mut file)
        .unwrap();
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn configure_linux_link_shims() {
    use std::env;
    use std::fs;
    use std::os::unix::fs::symlink;

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let shim_dir = out_dir.join("native-link-shims");
    let candidates = linux_library_search_dirs();
    let mut added = false;

    for lib_name in ["xcb-image", "xkbcommon-x11"] {
        if has_unversioned_lib(&candidates, lib_name) {
            continue;
        }

        let Some(versioned) = find_versioned_lib(&candidates, lib_name) else {
            continue;
        };

        fs::create_dir_all(&shim_dir).unwrap();
        let shim_path = shim_dir.join(format!("lib{lib_name}.so"));
        if shim_path.exists() {
            fs::remove_file(&shim_path).unwrap();
        }
        symlink(&versioned, &shim_path).unwrap_or_else(|err| {
            panic!(
                "create linker shim {} -> {} failed: {err}",
                shim_path.display(),
                versioned.display()
            )
        });
        added = true;
    }

    if added {
        println!("cargo:rustc-link-search=native={}", shim_dir.display());
    }
}

#[cfg(not(all(unix, not(target_os = "macos"))))]
fn configure_linux_link_shims() {}

#[cfg(all(unix, not(target_os = "macos")))]
fn has_unversioned_lib(search_dirs: &[PathBuf], lib_name: &str) -> bool {
    search_dirs
        .iter()
        .map(|dir| dir.join(format!("lib{lib_name}.so")))
        .any(|path| path.exists())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn find_versioned_lib(search_dirs: &[PathBuf], lib_name: &str) -> Option<PathBuf> {
    search_dirs.iter().find_map(|dir| {
        let prefix = format!("lib{lib_name}.so.");
        let mut entries = fs::read_dir(dir).ok()?.flatten().collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        entries.into_iter().map(|entry| entry.path()).find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(&prefix))
        })
    })
}

#[cfg(all(unix, not(target_os = "macos")))]
fn linux_library_search_dirs() -> Vec<PathBuf> {
    use std::collections::BTreeSet;

    let mut dirs = BTreeSet::new();
    for root in ["/usr/lib", "/lib"] {
        let root_path = Path::new(root);
        dirs.insert(root_path.to_path_buf());
        if let Ok(entries) = fs::read_dir(root_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir()
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.ends_with("-linux-gnu"))
                {
                    dirs.insert(path);
                }
            }
        }
    }
    if let Ok(extra) = std::env::var("LIBRARY_PATH") {
        for dir in extra.split(':').filter(|dir| !dir.is_empty()) {
            dirs.insert(PathBuf::from(dir));
        }
    }
    dirs.into_iter().collect()
}
