use super::*;
use tempfile::TempDir;

struct Fixture {
    dir: TempDir,
}

impl Fixture {
    fn new() -> Self {
        Fixture {
            dir: TempDir::new().unwrap(),
        }
    }

    fn lib_path(&self) -> PathBuf {
        self.dir.path().join("lib")
    }

    fn source(&self) -> PathBuf {
        self.dir.path().join("source.rs")
    }

    fn write_source(&self, contents: &str) -> PathBuf {
        let path = self.source();
        fs::write(&path, contents).unwrap();
        path
    }

    fn write_lib(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.lib_path().join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, contents).unwrap();
        path
    }

    fn determine(&self, macro_file_map: &HashMap<Ident, PathBuf>) -> HashSet<PathBuf> {
        determine_files(&self.source(), "algo_lib", &self.lib_path(), macro_file_map).unwrap()
    }
}

fn ident(name: &str) -> Ident {
    syn::parse_str(name).unwrap()
}

#[test]
fn follows_use_statements_transitively() {
    let fx = Fixture::new();
    fx.write_lib("lib.rs", "pub mod a;\npub mod b;\npub mod unused;\n");
    let a = fx.write_lib("a.rs", "use crate::b::helper;\npub struct Foo;\n");
    let b = fx.write_lib("b/mod.rs", "pub fn helper() {}\n");
    fx.write_lib("unused.rs", "pub struct Unused;\n");
    fx.write_source("use algo_lib::a::Foo;\nfn main() {}\n");

    assert_eq!(fx.determine(&HashMap::new()), HashSet::from([a, b]));
}

#[test]
fn resolves_grouped_and_renamed_imports() {
    let fx = Fixture::new();
    fx.write_lib("lib.rs", "pub mod a;\npub mod b;\n");
    let a = fx.write_lib("a.rs", "pub struct Foo;\n");
    let b = fx.write_lib("b.rs", "pub struct Bar;\n");
    fx.write_source("use algo_lib::{a::Foo, b as renamed};\nfn main() {}\n");

    assert_eq!(fx.determine(&HashMap::new()), HashSet::from([a, b]));
}

#[test]
fn resolves_paths_through_imported_names() {
    let fx = Fixture::new();
    fx.write_lib("lib.rs", "pub mod a;\n");
    let a = fx.write_lib("a.rs", "pub mod deep;\n");
    let deep = fx.write_lib("a/deep.rs", "pub fn f() {}\n");
    // `a` is only referenced through a use statement, `deep` only through a path
    fx.write_source("use algo_lib::a;\nfn main() {\n    a::deep::f();\n}\n");

    assert_eq!(fx.determine(&HashMap::new()), HashSet::from([a, deep]));
}

#[test]
fn resolves_fully_qualified_paths() {
    let fx = Fixture::new();
    fx.write_lib("lib.rs", "pub mod a;\n");
    let a = fx.write_lib("a.rs", "pub fn f() {}\n");
    fx.write_source("fn main() {\n    algo_lib::a::f();\n}\n");

    assert_eq!(fx.determine(&HashMap::new()), HashSet::from([a]));
}

#[test]
fn includes_files_defining_used_macros() {
    let fx = Fixture::new();
    fx.write_lib("lib.rs", "pub mod macros;\n");
    let macros = fx.write_lib(
        "macros.rs",
        "#[macro_export]\nmacro_rules! shout { () => {} }\n",
    );
    fx.write_source("use algo_lib::shout;\nfn main() {\n    shout!();\n}\n");
    let macro_file_map = HashMap::from([(ident("shout"), macros.clone())]);

    assert_eq!(fx.determine(&macro_file_map), HashSet::from([macros]));
}

#[test]
fn includes_macro_file_for_invocation_path() {
    let fx = Fixture::new();
    fx.write_lib("lib.rs", "pub mod macros;\n");
    let macros = fx.write_lib(
        "macros.rs",
        "#[macro_export]\nmacro_rules! shout { () => {} }\n",
    );
    fx.write_source("fn main() {\n    algo_lib::shout!();\n}\n");
    let macro_file_map = HashMap::from([(ident("shout"), macros.clone())]);

    assert_eq!(fx.determine(&macro_file_map), HashSet::from([macros]));
}

#[test]
fn ignores_bare_library_import() {
    let fx = Fixture::new();
    fx.write_lib("lib.rs", "pub mod a;\n");
    fx.write_lib("a.rs", "pub struct Foo;\n");
    fx.write_source("use algo_lib;\nfn main() {}\n");

    assert!(fx.determine(&HashMap::new()).is_empty());
}

#[test]
fn ignores_imports_from_other_crates() {
    let fx = Fixture::new();
    fx.write_lib("lib.rs", "pub mod a;\n");
    fx.write_lib("a.rs", "pub struct Foo;\n");
    fx.write_source("use std::a::Foo;\nfn main() {}\n");

    assert!(fx.determine(&HashMap::new()).is_empty());
}

#[test]
fn includes_files_referenced_from_lib_rs() {
    let fx = Fixture::new();
    fx.write_lib("lib.rs", "pub mod a;\nuse crate::a::Foo;\n");
    let a = fx.write_lib("a.rs", "pub struct Foo;\n");
    fx.write_source("fn main() {}\n");

    assert_eq!(fx.determine(&HashMap::new()), HashSet::from([a]));
}

#[test]
#[should_panic(expected = "Can't use glob imports")]
fn rejects_glob_imports() {
    let fx = Fixture::new();
    fx.write_lib("lib.rs", "pub mod a;\n");
    fx.write_lib("a.rs", "pub struct Foo;\n");
    fx.write_source("use algo_lib::a::*;\nfn main() {}\n");

    fx.determine(&HashMap::new());
}

#[test]
fn errors_on_unparsable_library_file() {
    let fx = Fixture::new();
    fx.write_lib("lib.rs", "pub mod a;\n");
    fx.write_lib("a.rs", "fn broken( {");
    fx.write_source("use algo_lib::a::Foo;\nfn main() {}\n");

    assert!(determine_files(&fx.source(), "algo_lib", &fx.lib_path(), &HashMap::new()).is_err());
}

#[test]
fn missing_files_are_skipped() {
    let fx = Fixture::new();
    fx.write_lib("lib.rs", "pub mod a;\n");
    fx.write_source("use algo_lib::a::Foo;\nfn main() {}\n");

    assert!(fx.determine(&HashMap::new()).is_empty());
}
