fn main() {
    println!("cargo:rerun-if-changed=build/windows/rojo-manifest.rc");
    println!("cargo:rerun-if-changed=build/windows/rojo.manifest");
    embed_resource::compile("build/windows/rojo-manifest.rc");
}
