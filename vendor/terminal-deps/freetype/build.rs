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

fn ensure_vendor_deps_ready() {
    if vendored_file_exists("zlib", "adler32.c")
        && vendored_file_exists("libpng", "png.c")
        && vendored_file_exists("freetype2", "include/freetype/freetype.h")
    {
        return;
    }
    let repo_root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap())
        .join("..")
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap();
    let script = repo_root
        .join("scripts")
        .join("bootstrap-terminal-vendor-deps.sh");
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

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let mut cfg = new_build();
    let build_dir = out_dir.join("zlib-build");
    fs::create_dir_all(&build_dir).unwrap();
    cfg.out_dir(&build_dir);
    cfg.file("zlib/adler32.c")
        .file("zlib/compress.c")
        .file("zlib/crc32.c")
        .file("zlib/deflate.c")
        .file("zlib/gzclose.c")
        .file("zlib/gzlib.c")
        .file("zlib/gzread.c")
        .file("zlib/gzwrite.c")
        .file("zlib/inflate.c")
        .file("zlib/infback.c")
        .file("zlib/inftrees.c")
        .file("zlib/inffast.c")
        .file("zlib/trees.c")
        .file("zlib/uncompr.c")
        .file("zlib/zutil.c");
    cfg.include("zlib");
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

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let mut cfg = new_build();
    let build_dir = out_dir.join("png-build");
    fs::create_dir_all(&build_dir).unwrap();
    cfg.out_dir(&build_dir);

    cfg.file("libpng/png.c")
        .file("libpng/pngerror.c")
        .file("libpng/pngget.c")
        .file("libpng/pngmem.c")
        .file("libpng/pngpread.c")
        .file("libpng/pngread.c")
        .file("libpng/pngrio.c")
        .file("libpng/pngrtran.c")
        .file("libpng/pngrutil.c")
        .file("libpng/pngset.c")
        .file("libpng/pngsimd.c")
        .file("libpng/pngtrans.c")
        .file("libpng/pngwio.c")
        .file("libpng/pngwrite.c")
        .file("libpng/pngwtran.c")
        .file("libpng/pngwutil.c");

    // This vendored build uses libpng's prebuilt configuration header, which
    // disables target-specific acceleration paths (`PNG_TARGET_SPECIFIC_CODE=0`).
    // Compiling the ARM NEON sources anyway breaks Apple Silicon builds because
    // those translation units are intended to be included from arm_init.c under
    // matching libpng feature defines, not built unconditionally as standalone
    // files.

    cfg.include("zlib");
    cfg.include("libpng");
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
        PathBuf::from("libpng/scripts/pnglibconf.h.prebuilt"),
        PathBuf::from("libpng/pnglibconf.h.prebuilt"),
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

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let mut cfg = new_build();
    let build_dir = out_dir.join("freetype-build");
    fs::create_dir_all(&build_dir).unwrap();
    cfg.out_dir(&build_dir);
    cfg.include("zlib");
    cfg.include("libpng");
    cfg.include(out_dir.join("png-build"));

    fs::create_dir_all(build_dir.join("freetype2/include/freetype/config")).unwrap();
    cfg.include(format!("{}/freetype2/include", build_dir.display()));
    cfg.include("freetype2/include");
    cfg.define("FT2_BUILD_LIBRARY", None);

    let target = env::var("TARGET").unwrap();

    fs::write(
        build_dir.join("freetype2/include/freetype/config/ftoption.h"),
        fs::read_to_string("freetype2/include/freetype/config/ftoption.h")
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
        cfg.file(format!("freetype2/src/{}", f));
    }

    if target.contains("windows") {
        cfg.file("freetype2/builds/windows/ftdebug.c");
    } else {
        cfg.file("freetype2/src/base/ftdebug.c");
    }

    cfg.compile("freetype");

    // These cause DEP_FREETYPE_INCLUDE and DEP_FREETYPE_LIB to be
    // defined in the harfbuzz/build.rs
    println!(
        "cargo:include={}/include/freetype2;{}/freetype2/include",
        build_dir.display(),
        std::env::current_dir().unwrap().display()
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
