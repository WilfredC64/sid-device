const USE_NEW_FILTER: bool = true;

fn main() -> miette::Result<()> {
    let mut src = vec![
        "src/resid10/dac.cc",
        "src/resid10/envelope.cc",
        "src/resid10/extfilt.cc",
        "src/resid10/pot.cc",
        "src/resid10/sid.cc",
        "src/resid10/version.cc",
        "src/resid10/voice.cc",
        "src/resid10/wave.cc",
        ];

    if USE_NEW_FILTER {
        src.push("src/resid10/filter8580new.cc");
    } else {
        src.push("src/resid10/filter.cc");
    }

    let path = std::path::PathBuf::from("src");
    autocxx_build::Builder::new("src/lib.rs", [&path]).build()?
        .define("VERSION", Some("\"1.0\""))
        .define("NEW_8580_FILTER", Some(if USE_NEW_FILTER {"1"} else {"0"}))
        .files(src)
        .flag_if_supported("-std=c++14")
        .flag_if_supported("-Wno-psabi")
        .warnings(false)
        .compile("resid");

    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/resid10/");
    Ok(())
}
