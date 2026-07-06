const USE_NEW_FILTER: bool = true;

fn main() {
    let mut src = vec![
        "src/resid10/dac.cc",
        "src/resid10/envelope.cc",
        "src/resid10/extfilt.cc",
        "src/resid10/pot.cc",
        "src/resid10/sid.cc",
        "src/resid10/version.cc",
        "src/resid10/voice.cc",
        "src/resid10/wave.cc",
        "src/resid_bridge.cc",
    ];

    if USE_NEW_FILTER {
        src.push("src/resid10/filter8580new.cc");
    } else {
        src.push("src/resid10/filter.cc");
    }

    let mut builder = cxx_build::bridge("src/lib.rs");
    let build = builder
        .files(src)
        .include("src")
        .define("VERSION", Some("\"1.0\""))
        .define("NEW_8580_FILTER", Some(if USE_NEW_FILTER { "1" } else { "0" }))
        .flag_if_supported("-Wno-psabi")
        .warnings(false);

    #[cfg(target_os = "macos")]
    build.flag_if_supported("-std=c++14");

    build.compile("resid");

    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/resid_bridge.h");
    println!("cargo:rerun-if-changed=src/resid_bridge.cc");
    println!("cargo:rerun-if-changed=src/resid10/");
}
