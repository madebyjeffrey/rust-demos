fn main() {
    println!("cargo:rerun-if-changed=ios/scene.m");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("ios") {
        cc::Build::new()
            .file("ios/scene.m")
            .flag("-fobjc-arc")
            .compile("galaxy_ios_scene");
    }
}
