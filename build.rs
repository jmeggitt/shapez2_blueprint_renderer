use std::env;
use std::fs::File;
use std::path::PathBuf;

use gl_generator::{Api, Fallbacks, Profile, Registry, StructGenerator};


fn main() {
    let dest = PathBuf::from(&env::var("OUT_DIR").unwrap());

    let mut file = File::create(dest.join("gl_bindings.rs")).unwrap();
    Registry::new(Api::Gles2, (3, 3), Profile::Core, Fallbacks::All, [])
        .write_bindings(StructGenerator, &mut file)
        .unwrap();
}

// fn main() {}
