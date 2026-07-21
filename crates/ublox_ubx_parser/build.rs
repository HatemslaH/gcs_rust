use std::env;
use std::path::PathBuf;

fn main() {
    // bindgen ищет libclang; на Windows часто нужен явный путь к LLVM.
    if env::var_os("LIBCLANG_PATH").is_none() {
        let llvm_bin = PathBuf::from(r"C:\Program Files\LLVM\bin");
        if llvm_bin.join("libclang.dll").is_file() {
            // SAFETY: только для build-скрипта, до вызова bindgen.
            unsafe {
                env::set_var("LIBCLANG_PATH", &llvm_bin);
            }
        }
    }

    let native = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("native");
    let ubx_include = native.join("u-blox-bg").join("include").join("ubx");
    let wrapper = native.join("wrapper.h");

    cc::Build::new()
        .file(native.join("ubx_parser.c"))
        .file(native.join("u-blox-bg/src/ubx/ubx-default-msg.c"))
        .file(native.join("u-blox-bg/src/ubx/ubx-nav-pvt.c"))
        .file(native.join("u-blox-bg/src/ubx/ubx-nav-svin.c"))
        .file(native.join("u-blox-bg/src/ubx/ubx-cfg-valset.c"))
        .file(native.join("u-blox-bg/src/ubx/ubx-cfg-valdel.c"))
        .file(native.join("u-blox-bg/src/ubx/ubx-cfg-rst.c"))
        .include(&native)
        .include(&ubx_include)
        .warnings(false)
        .compile("ublox_ubx_parser_native");

    let bindings = bindgen::Builder::default()
        .header(wrapper.to_str().expect("utf-8 path"))
        .clang_arg(format!("-I{}", native.display()))
        .clang_arg(format!("-I{}", ubx_include.display()))
        .allowlist_function("ublox_ubx_parser_init")
        .allowlist_function("ubx_.*")
        .allowlist_type("ubx_.*")
        .allowlist_var("UBX_.*")
        .default_enum_style(bindgen::EnumVariation::Consts)
        .prepend_enum_name(false)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("не удалось сгенерировать bindgen-биндинги");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("не удалось записать bindings.rs");

    println!("cargo:rerun-if-changed=native/wrapper.h");
    println!("cargo:rerun-if-changed=native/ublox_ubx_parser.h");
    println!("cargo:rerun-if-changed=native/ubx_parser.c");
    println!("cargo:rerun-if-changed=native/u-blox-bg");
}
