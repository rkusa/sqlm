#[rustversion::nightly]
fn main() {
    // `&'static str` support stopped working on nightly, thus disabled for now
    // println!("cargo:rustc-cfg=nightly_column_names");
    println!("cargo:rustc-cfg=nightly_fail_tests");
    println!("cargo::rustc-check-cfg=cfg(nightly_column_names)");
    println!("cargo::rustc-check-cfg=cfg(nightly_fail_tests)");
}

#[rustversion::not(nightly)]
fn main() {
    println!("cargo::rustc-check-cfg=cfg(nightly_column_names)");
    println!("cargo::rustc-check-cfg=cfg(nightly_fail_tests)");
}
