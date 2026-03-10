use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    configure_linux_link_shims();

    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let repo_root = manifest_dir
        .join("..")
        .join("..")
        .canonicalize()
        .expect("canonicalize repo root");
    println!(
        "cargo:rerun-if-changed={}",
        repo_root
            .join("scripts")
            .join("bootstrap-terminal-vendor-deps.sh")
            .display()
    );
    ensure_vendored_native_deps(&repo_root);

    #[cfg(windows)]
    {
        let asset_root = manifest_dir.join("assets");
        configure_windows_assets(&repo_root, &asset_root);
    }
    #[cfg(target_os = "macos")]
    {
        let asset_root = manifest_dir.join("assets");
        configure_macos_assets(&repo_root, &asset_root);
    }
}

fn ensure_vendored_native_deps(repo_root: &Path) {
    let script = repo_root
        .join("scripts")
        .join("bootstrap-terminal-vendor-deps.sh");
    let status = Command::new("bash")
        .arg(&script)
        .arg("--quiet")
        .current_dir(repo_root)
        .status()
        .unwrap_or_else(|err| panic!("run {} failed: {err}", script.display()));
    if !status.success() {
        panic!(
            "bootstrap terminal vendor deps failed with status {status}: {}",
            script.display()
        );
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn configure_linux_link_shims() {
    use std::fs;
    use std::os::unix::fs::symlink;

    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
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

        fs::create_dir_all(&shim_dir).expect("create native-link-shims");
        let shim_path = shim_dir.join(format!("lib{lib_name}.so"));
        if shim_path.exists() {
            fs::remove_file(&shim_path).expect("replace native-link-shim");
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
    use std::fs;

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
    use std::fs;

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

#[cfg(windows)]
fn copy_asset(src: &Path, dest: &Path) {
    use anyhow::Context as _;

    println!("cargo:rerun-if-changed={}", src.display());
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .context(format!("create asset dir {}", parent.display()))
            .unwrap();
    }
    std::fs::copy(src, dest)
        .context(format!("copy {} -> {}", src.display(), dest.display()))
        .unwrap();
}

#[cfg(windows)]
fn configure_windows_assets(repo_root: &Path, asset_root: &Path) {
    use std::io::Write;

    let profile = std::env::var("PROFILE").expect("PROFILE");
    let exe_output_dir = build_target_dir(repo_root, &profile);
    let windows_dir = asset_root.join("windows");

    let conhost_dir = windows_dir.join("conhost");
    for name in &["conpty.dll", "OpenConsole.exe"] {
        let dest_name = exe_output_dir.join(name);
        let src_name = conhost_dir.join(name);
        copy_asset(&src_name, &dest_name);
    }

    let angle_dir = windows_dir.join("angle");
    for name in &["libEGL.dll", "libGLESv2.dll"] {
        let dest_name = exe_output_dir.join(name);
        let src_name = angle_dir.join(name);
        copy_asset(&src_name, &dest_name);
    }

    let dest_mesa = exe_output_dir.join("mesa");
    let dest_name = dest_mesa.join("opengl32.dll");
    let src_name = windows_dir.join("mesa").join("opengl32.dll");
    copy_asset(&src_name, &dest_name);

    let version = resolve_windows_version(repo_root);
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    let rcfile_name = out_dir.join("resource.rc");
    let mut rcfile = std::fs::File::create(&rcfile_name).expect("create rc file");
    println!(
        "cargo:rerun-if-changed={}",
        windows_dir.join("terminal.ico").display()
    );
    write!(
        rcfile,
        r#"
#include <winres.h>
#define IDI_ICON 0x101
1 RT_MANIFEST "{win}\\manifest.manifest"
IDI_ICON ICON "{win}\\terminal.ico"
VS_VERSION_INFO VERSIONINFO
FILEVERSION     1,0,0,0
PRODUCTVERSION  1,0,0,0
FILEFLAGSMASK   VS_FFI_FILEFLAGSMASK
FILEFLAGS       0
FILEOS          VOS__WINDOWS32
FILETYPE        VFT_APP
FILESUBTYPE     VFT2_UNKNOWN
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904E4"
        BEGIN
            VALUE "CompanyName",      "Chatminal\0"
            VALUE "FileDescription",  "Chatminal Desktop\0"
            VALUE "FileVersion",      "{version}\0"
            VALUE "InternalName",     "chatminal-desktop\0"
            VALUE "OriginalFilename", "chatminal-desktop.exe\0"
            VALUE "ProductName",      "Chatminal\0"
            VALUE "ProductVersion",   "{version}\0"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x409, 1252
    END
END
"#,
        win = windows_dir.display().to_string().replace("\\", "\\\\"),
        version = version,
    )
    .expect("write rc file");

    let target = std::env::var("TARGET").expect("TARGET");
    if let Some(tool) = cc::windows_registry::find_tool(target.as_str(), "cl.exe") {
        for (key, value) in tool.env() {
            std::env::set_var(key, value);
        }
    }
    embed_resource::compile(rcfile_name);
}

#[cfg(windows)]
fn resolve_windows_version(repo_root: &Path) -> String {
    let tag_path = repo_root.join(".tag");
    if let Ok(tag) = std::fs::read(&tag_path) {
        if let Ok(tag) = String::from_utf8(tag) {
            println!("cargo:rerun-if-changed={}", tag_path.display());
            let trimmed = tag.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    let output = Command::new("git")
        .args([
            "-C",
            repo_root.to_str().expect("repo root utf8"),
            "-c",
            "core.abbrev=8",
            "show",
            "-s",
            "--format=%cd-%h",
            "--date=format:%Y%m%d-%H%M%S",
        ])
        .output();
    output
        .ok()
        .filter(|value| value.status.success())
        .map(|value| String::from_utf8_lossy(&value.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "UNKNOWN".to_string())
}

#[cfg(target_os = "macos")]
fn configure_macos_assets(repo_root: &Path, asset_root: &Path) {
    use anyhow::Context as _;

    let profile = std::env::var("PROFILE").expect("PROFILE");
    let src_plist = asset_root
        .join("macos")
        .join("Chatminal.app")
        .join("Contents")
        .join("Info.plist");
    let dest_plist = build_target_dir(repo_root, &profile).join("Info.plist");
    println!("cargo:rerun-if-changed={}", src_plist.display());

    std::fs::copy(&src_plist, &dest_plist)
        .context(format!(
            "copy {} -> {}",
            src_plist.display(),
            dest_plist.display()
        ))
        .unwrap();
}

#[cfg(any(windows, target_os = "macos"))]
fn build_target_dir(repo_root: &Path, profile: &str) -> PathBuf {
    std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root.join("target"))
        .join(profile)
}
