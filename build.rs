use gl_generator::{Api, Fallbacks, GlobalGenerator, Profile, Registry};
use std::{env, fs::File, path::Path};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let mut file = File::create(Path::new(&out_dir).join("bindings.rs")).unwrap();

    Registry::new(
        Api::Gl,
        (4, 6), // Generate OpenGL 4.6 bindings
        Profile::Core,
        Fallbacks::All,
        ["GL_ARB_bindless_texture"],
    )
    .write_bindings(GlobalGenerator, &mut file)
    .unwrap();
}
