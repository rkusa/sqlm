#[rustversion::nightly]
fn main() {
    println!("cargo:rustc-cfg=nightly_column_names");
    println!("cargo::rustc-check-cfg=cfg(nightly_column_names)");
}

#[rustversion::not(nightly)]
fn main() {
    println!("cargo::rustc-check-cfg=cfg(nightly_column_names)");
}
