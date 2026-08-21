use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn write(dir: &TempDir, name: &str, contents: &str) {
    let path = dir.path().join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

#[test]
fn builds_single_file_source() {
    let dir = TempDir::new().unwrap();
    write(&dir, "source.rs", "use algo_lib::a::Foo;\nfn main() {}\n");
    write(&dir, "lib/src/lib.rs", "pub mod a;\npub mod unused;\n");
    write(&dir, "lib/src/a.rs", "use crate::b::g;\npub struct Foo;\n");
    write(&dir, "lib/src/b.rs", "pub fn g() {}\n");
    write(&dir, "lib/src/unused.rs", "pub struct Unused;\n");
    let output = dir.path().join("out.rs");

    let status = Command::new(env!("CARGO_BIN_EXE_source-builder"))
        .arg(dir.path().join("source.rs"))
        .args(["--library", "algo_lib"])
        .arg("--path")
        .arg(dir.path().join("lib"))
        .arg("--output")
        .arg(&output)
        .status()
        .unwrap();

    assert!(status.success());
    let content = fs::read_to_string(&output).unwrap();
    assert_eq!(
        content,
        "use crate::algo_lib::a::Foo;\nfn main() {}\npub mod algo_lib {\n    pub mod a {\n        use crate::algo_lib::b::g;\n        pub struct Foo;\n    }\n    pub mod b {\n        pub fn g() {}\n    }\n}\n"
    );
}

#[test]
fn fails_when_source_is_missing() {
    let dir = TempDir::new().unwrap();
    write(&dir, "lib/src/lib.rs", "");

    let status = Command::new(env!("CARGO_BIN_EXE_source-builder"))
        .arg(dir.path().join("missing.rs"))
        .args(["--library", "algo_lib"])
        .arg("--path")
        .arg(dir.path().join("lib"))
        .arg("--output")
        .arg(dir.path().join("out.rs"))
        .status()
        .unwrap();

    assert!(!status.success());
}
