use std::fs;
use std::path::PathBuf;

const ENV_WARNING: &'static str = r#"Error while building blpapi-sys.

    Cannot find 'BLPAPI_LIB' environment variable.

    You can download blpapi binaries from bloomberg at:
    https://www.bloomberg.com/professional/support/api-library/

    Once extracted, the BLPAPI_LIB environment variable should point to the
    corresponding lib dir:

    - windows: <EXTRACT_PATH>\lib
    - linux: <EXTRACT_PATH>/Linux"
"#;

fn main() {
    let lib_dir: String = if cfg!(feature = "bundled") {
        let mut dir: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        
        dir.pop();
        dir.push("vendor");

        for entry in fs::read_dir(dir.as_path()).expect("Failed to read `vendor/` dir...") {
            let entry: fs::DirEntry = entry.expect("Failed to read entry in `vendor/` dir...");
            let path: PathBuf = entry.path();

            if path.is_dir() {
                let dir_name: std::borrow::Cow<str> =
                    path.file_name().unwrap_or_default().to_string_lossy();

                if cfg!(windows) && dir_name.ends_with("windows") {
                    dir.push(path);
                    dir.push("lib");

                    break;
                } else if cfg!(unix) && dir_name.ends_with("linux") {
                    dir.push(path);
                    dir.push("Linux");

                    break;
                }
            }
        }

        dir.into_os_string().into_string().unwrap()
    } else {
        std::env::var("BLPAPI_LIB").expect(ENV_WARNING)
    };

    println!("cargo:rustc-link-search={}", lib_dir);
    println!("cargo:rustc-link-lib=blpapi3_64");
}
