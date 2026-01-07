use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let shaders = [
        ("assets/shader.vert", "shader.vert.spv"),
        ("assets/shader.frag", "shader.frag.spv"),
    ];

    for (src, dst) in shaders {
        let out = out_dir.join(dst);
        let status = Command::new("glslc")
            .args([src, "-o"])
            .arg(&out)
            .status()
            .expect("failed to run glslc");

        assert!(status.success());
        println!("cargo:rerun-if-changed={src}");
    }
}
