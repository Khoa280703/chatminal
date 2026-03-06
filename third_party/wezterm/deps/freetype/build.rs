use std::path::{Path, PathBuf};
use std::{env, fs};

fn new_build() -> cc::Build {
    let mut cfg = cc::Build::new();
    cfg.warnings(false);
    cfg.flag_if_supported("-fno-stack-check");
    cfg.flag_if_supported("-Wno-deprecated-non-prototype");
    cfg
}

fn vendored_file_exists(dir: &str, sentinel: &str) -> bool {
    Path::new(dir).join(sentinel).is_file()
}

fn repo_root() -> PathBuf {
    PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap())
        .join("..")
        .join("..")
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap()
}

fn first_party_vendor_root() -> PathBuf {
    repo_root().join("vendor").join("wezterm-deps")
}

fn resolve_freetype_vendor_dir(local_dir: &str, sentinel: &str) -> PathBuf {
    let local = PathBuf::from(local_dir);
    if local.join(sentinel).is_file() {
        return local;
    }

    let external = first_party_vendor_root().join("freetype").join(local_dir);
    if external.join(sentinel).is_file() {
        return external;
    }

    ensure_vendor_deps_ready();

    if external.join(sentinel).is_file() {
        return external;
    }
    if local.join(sentinel).is_file() {
        return local;
    }

    panic!(
        "missing vendored freetype dependency {local_dir} (sentinel {sentinel})"
    );
}

fn ensure_vendor_deps_ready() {
    let local_ready = vendored_file_exists("zlib", "adler32.c")
        && vendored_file_exists("libpng", "png.c")
        && vendored_file_exists("freetype2", "include/freetype/freetype.h");
    let first_party_root = first_party_vendor_root();
    let external_ready = first_party_root.join("freetype/zlib/adler32.c").is_file()
        && first_party_root.join("freetype/libpng/png.c").is_file()
        && first_party_root
            .join("freetype/freetype2/include/freetype/freetype.h")
            .is_file();
    if local_ready || external_ready {
        return;
    }
    let script = repo_root().join("scripts").join("bootstrap-wezterm-vendor-deps.sh");
    let status = std::process::Command::new("bash")
        .arg(script)
        .arg("--quiet")
        .status();
    if !status.map(|value| value.success()).unwrap_or(false) {
        git_submodule_update();
    }
}

fn zlib() {
    ensure_vendor_deps_ready();
    let zlib_dir = resolve_freetype_vendor_dir("zlib", "adler32.c");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let mut cfg = new_build();
    let build_dir = out_dir.join("zlib-build");
    fs::create_dir_all(&build_dir).unwrap();
    cfg.out_dir(&build_dir);
    for file in [
        "adler32.c",
        "compress.c",
        "crc32.c",
        "deflate.c",
        "gzclose.c",
        "gzlib.c",
        "gzread.c",
        "gzwrite.c",
        "inflate.c",
        "infback.c",
        "inftrees.c",
        "inffast.c",
        "trees.c",
        "uncompr.c",
        "zutil.c",
    ] {
        cfg.file(zlib_dir.join(file));
    }
    cfg.include(&zlib_dir);
    cfg.define("HAVE_SYS_TYPES_H", None);
    cfg.define("HAVE_STDINT_H", None);
    cfg.define("HAVE_STDDEF_H", None);
    let target = env::var("TARGET").unwrap();
    if !target.contains("windows") {
        cfg.define("_LARGEFILE64_SOURCE", Some("1"));
    }
    cfg.compile("z");
}

fn libpng() {
    ensure_vendor_deps_ready();
    let zlib_dir = resolve_freetype_vendor_dir("zlib", "adler32.c");
    let libpng_dir = resolve_freetype_vendor_dir("libpng", "png.c");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let mut cfg = new_build();
    let build_dir = out_dir.join("png-build");
    fs::create_dir_all(&build_dir).unwrap();
    cfg.out_dir(&build_dir);

    for file in [
        "png.c",
        "pngerror.c",
        "pngget.c",
        "pngmem.c",
        "pngpread.c",
        "pngread.c",
        "pngrio.c",
        "pngrtran.c",
        "pngrutil.c",
        "pngset.c",
        "pngsimd.c",
        "pngtrans.c",
        "pngwio.c",
        "pngwrite.c",
        "pngwtran.c",
        "pngwutil.c",
    ] {
        cfg.file(libpng_dir.join(file));
    }

    if let Ok(arch) = env::var("CARGO_CFG_TARGET_ARCH") {
        match arch.as_str() {
            "aarch64" | "arm" => {
                cfg.file(libpng_dir.join("arm/arm_init.c"))
                    .file(libpng_dir.join("arm/filter_neon.S"))
                    .file(libpng_dir.join("arm/filter_neon_intrinsics.c"))
                    .file(libpng_dir.join("arm/palette_neon_intrinsics.c"));
            }
            _ => {}
        }
    }

    cfg.include(&zlib_dir);
    cfg.include(&libpng_dir);
    cfg.include(&build_dir);
    cfg.define("HAVE_SYS_TYPES_H", None);
    cfg.define("HAVE_STDINT_H", None);
    cfg.define("HAVE_STDDEF_H", None);
    let target = env::var("TARGET").unwrap();
    if target.contains("powerpc64") {
        cfg.define("PNG_POWERPC_VSX_OPT", Some("0"));
    }
    if !target.contains("windows") {
        cfg.define("_LARGEFILE64_SOURCE", Some("1"));
    }

    let prebuilt_candidates = [
        libpng_dir.join("scripts/pnglibconf.h.prebuilt"),
        libpng_dir.join("pnglibconf.h.prebuilt"),
    ];
    let prebuilt_config = prebuilt_candidates
        .iter()
        .find(|path| path.is_file())
        .unwrap();
    fs::write(
        build_dir.join("pnglibconf.h"),
        fs::read_to_string(prebuilt_config).unwrap(),
    )
    .unwrap();

    cfg.compile("png");
}

fn freetype() {
    ensure_vendor_deps_ready();
    let zlib_dir = resolve_freetype_vendor_dir("zlib", "adler32.c");
    let libpng_dir = resolve_freetype_vendor_dir("libpng", "png.c");
    let freetype_dir =
        resolve_freetype_vendor_dir("freetype2", "include/freetype/freetype.h");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let mut cfg = new_build();
    let build_dir = out_dir.join("freetype-build");
    fs::create_dir_all(&build_dir).unwrap();
    cfg.out_dir(&build_dir);
    cfg.include(&zlib_dir);
    cfg.include(&libpng_dir);
    cfg.include(out_dir.join("png-build"));

    fs::create_dir_all(build_dir.join("freetype2/include/freetype/config")).unwrap();
    cfg.include(format!("{}/freetype2/include", build_dir.display()));
    cfg.include(freetype_dir.join("include"));
    cfg.define("FT2_BUILD_LIBRARY", None);

    let target = env::var("TARGET").unwrap();

    fs::write(
        build_dir.join("freetype2/include/freetype/config/ftoption.h"),
        fs::read_to_string(freetype_dir.join("include/freetype/config/ftoption.h"))
            .unwrap()
            .replace(
                "/* #define FT_CONFIG_OPTION_ERROR_STRINGS */",
                "#define FT_CONFIG_OPTION_ERROR_STRINGS",
            )
            .replace(
                "/* #define FT_CONFIG_OPTION_SYSTEM_ZLIB */",
                "#define FT_CONFIG_OPTION_SYSTEM_ZLIB",
            )
            .replace(
                "/* #define FT_CONFIG_OPTION_USE_PNG */",
                "#define FT_CONFIG_OPTION_USE_PNG",
            )
            .replace(
                "#define TT_CONFIG_OPTION_SUBPIXEL_HINTING  2",
                "#define TT_CONFIG_OPTION_SUBPIXEL_HINTING  3",
            )
            .replace(
                "/* #define PCF_CONFIG_OPTION_LONG_FAMILY_NAMES */",
                "#define PCF_CONFIG_OPTION_LONG_FAMILY_NAMES",
            )
            .replace(
                "/* #define FT_CONFIG_OPTION_SUBPIXEL_RENDERING */",
                "#define FT_CONFIG_OPTION_SUBPIXEL_RENDERING",
            ),
    )
    .unwrap();

    for f in [
        "autofit/autofit.c",
        "base/ftbase.c",
        "base/ftbbox.c",
        "base/ftbdf.c",
        "base/ftbitmap.c",
        "base/ftcid.c",
        "base/ftfstype.c",
        "base/ftgasp.c",
        "base/ftglyph.c",
        "base/ftgxval.c",
        "base/ftinit.c",
        "base/ftmm.c",
        "base/ftotval.c",
        "base/ftpatent.c",
        "base/ftpfr.c",
        "base/ftstroke.c",
        "base/ftsynth.c",
        "base/ftsystem.c",
        "base/fttype1.c",
        "base/ftwinfnt.c",
        "bdf/bdf.c",
        "bzip2/ftbzip2.c",
        "cache/ftcache.c",
        "cff/cff.c",
        "cid/type1cid.c",
        "gzip/ftgzip.c",
        "lzw/ftlzw.c",
        "pcf/pcf.c",
        "pfr/pfr.c",
        "psaux/psaux.c",
        "pshinter/pshinter.c",
        "psnames/psnames.c",
        "raster/raster.c",
        "sdf/ftbsdf.c",
        "sdf/ftsdf.c",
        "sdf/ftsdfcommon.c",
        "sdf/ftsdfrend.c",
        "sfnt/sfnt.c",
        "smooth/smooth.c",
        "svg/ftsvg.c",
        "truetype/truetype.c",
        "type1/type1.c",
        "type42/type42.c",
        "winfonts/winfnt.c",
    ]
    .iter()
    {
        cfg.file(freetype_dir.join("src").join(f));
    }

    if target.contains("windows") {
        cfg.file(freetype_dir.join("builds/windows/ftdebug.c"));
    } else {
        cfg.file(freetype_dir.join("src/base/ftdebug.c"));
    }

    cfg.compile("freetype");

    // These cause DEP_FREETYPE_INCLUDE and DEP_FREETYPE_LIB to be
    // defined in the harfbuzz/build.rs
    let freetype_include_root = freetype_dir
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    println!(
        "cargo:include={}/include/freetype2;{}/freetype2/include",
        build_dir.display(),
        freetype_include_root.display()
    );
    println!("cargo:lib={}", build_dir.display());
}

fn git_submodule_update() {
    let _ = std::process::Command::new("git")
        .args(&["submodule", "update", "--init"])
        .status();
}

fn main() {
    zlib();
    libpng();
    freetype();
    let out_dir = env::var("OUT_DIR").unwrap();
    println!("cargo:outdir={}", out_dir);
    println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=10.12");
}
