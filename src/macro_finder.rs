use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use syn::visit::Visit;
use syn::{Ident, ItemMacro, visit};
use walkdir::WalkDir;

struct MacroFinder<'a> {
    file: &'a Path,
    map: &'a mut HashMap<Ident, PathBuf>,
}
impl<'a> MacroFinder<'a> {
    pub fn new(file: &'a Path, map: &'a mut HashMap<Ident, PathBuf>) -> Self {
        MacroFinder { file, map }
    }
}
impl<'ast> Visit<'ast> for MacroFinder<'_> {
    fn visit_item_macro(&mut self, i: &'ast ItemMacro) {
        if i.attrs.iter().any(|a| a.path().is_ident("macro_export"))
            && let Some(ident) = i.ident.to_owned()
        {
            self.map.insert(ident, self.file.to_path_buf());
        }
        visit::visit_item_macro(self, i);
    }
}

pub fn precompute_macro_file_map(root: &PathBuf) -> Result<HashMap<Ident, PathBuf>, anyhow::Error> {
    let mut map = HashMap::new();
    for entry in WalkDir::new(root) {
        let entry = entry?;
        if !entry.path().extension().is_some_and(|s| s == "rs") {
            continue;
        }
        let contents = syn::parse_file(fs::read_to_string(entry.path())?.as_str())?;
        MacroFinder::new(entry.path(), &mut map).visit_file(&contents);
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use tempfile::TempDir;

    fn write(dir: &TempDir, name: &str, contents: &str) -> PathBuf {
        let path = dir.path().join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, contents).unwrap();
        path
    }

    fn names(map: &HashMap<Ident, PathBuf>) -> HashSet<String> {
        map.keys().map(|i| i.to_string()).collect()
    }

    #[test]
    fn finds_exported_macros_in_nested_files() {
        let dir = TempDir::new().unwrap();
        write(&dir, "lib.rs", "pub mod a;\npub mod b;\n");
        let a = write(
            &dir,
            "a.rs",
            "#[macro_export]\nmacro_rules! outer { () => {} }\n",
        );
        let b = write(
            &dir,
            "b/mod.rs",
            "#[macro_export]\nmacro_rules! inner { () => {} }\n",
        );

        let map = precompute_macro_file_map(&dir.path().to_path_buf()).unwrap();

        assert_eq!(names(&map), HashSet::from(["outer".into(), "inner".into()]));
        assert_eq!(map[&parse_ident("outer")], a);
        assert_eq!(map[&parse_ident("inner")], b);
    }

    #[test]
    fn ignores_macros_without_export_attribute() {
        let dir = TempDir::new().unwrap();
        write(&dir, "a.rs", "macro_rules! private { () => {} }\n");

        let map = precompute_macro_file_map(&dir.path().to_path_buf()).unwrap();

        assert!(map.is_empty());
    }

    #[test]
    fn ignores_macro_invocations_without_ident() {
        let dir = TempDir::new().unwrap();
        write(&dir, "a.rs", "#[macro_export]\ninclude!(\"other.rs\");\n");

        let map = precompute_macro_file_map(&dir.path().to_path_buf()).unwrap();

        assert!(map.is_empty());
    }

    #[test]
    fn ignores_non_rust_files() {
        let dir = TempDir::new().unwrap();
        write(
            &dir,
            "notes.txt",
            "#[macro_export]\nmacro_rules! ignored { () => {} }\n",
        );
        write(&dir, "Cargo.toml", "[package]\nname = \"x\"\n");

        let map = precompute_macro_file_map(&dir.path().to_path_buf()).unwrap();

        assert!(map.is_empty());
    }

    #[test]
    fn errors_on_unparsable_file() {
        let dir = TempDir::new().unwrap();
        write(&dir, "a.rs", "fn broken( {");

        assert!(precompute_macro_file_map(&dir.path().to_path_buf()).is_err());
    }

    #[test]
    fn errors_on_missing_root() {
        let dir = TempDir::new().unwrap();

        assert!(precompute_macro_file_map(&dir.path().join("missing")).is_err());
    }

    fn parse_ident(name: &str) -> Ident {
        syn::parse_str(name).unwrap()
    }
}
