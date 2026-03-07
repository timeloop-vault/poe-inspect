use cmake::Config;

fn main() {
    let dst = Config::new("ooz")
        .build_target("libooz")
        // Seg Faults if debug info is not included and crosscompiling
        .profile("RelWithDebInfo")
        .build();

    let build_dir = format!("{}/build", dst.display());
    println!("cargo:rustc-link-search=native={build_dir}");
    // MSVC puts outputs in a profile subdirectory (e.g. RelWithDebInfo/)
    println!("cargo:rustc-link-search=native={build_dir}/RelWithDebInfo");
    println!("cargo:rustc-link-lib=static=libooz");
}
