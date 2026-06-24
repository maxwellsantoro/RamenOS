use std::env;

fn main() {
    for key in [
        "RAMEN_GIT_SHA",
        "RAMEN_STORAGE_MANIFEST_SHA256",
        "RAMEN_MACHINE_ID",
        "RAMEN_KERNEL_EFI_SHA256",
        "RAMEN_INIT_IMG_SHA256",
    ] {
        let value = env::var(key).unwrap_or_else(|_| "unknown".to_string());
        println!("cargo:rustc-env={key}={value}");
    }
}
