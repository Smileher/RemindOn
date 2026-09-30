fn main() {
    println!("cargo:rerun-if-env-changed=REMINDON_STORE_BUILD");
    if std::env::var("REMINDON_STORE_BUILD").as_deref() == Ok("1") {
        println!("cargo:rustc-env=REMINDON_STORE_BUILD=1");
    }
    tauri_build::build()
}
