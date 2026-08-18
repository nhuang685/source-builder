use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use syn::visit::Visit;
use syn::{Ident, ItemMacro, visit};
use walkdir::WalkDir;

struct MacroFinder<'a> {
    pub file: &'a Path,
    pub map: &'a mut HashMap<Ident, PathBuf>,
}
impl<'a> MacroFinder<'a> {
    pub fn new(file: &'a Path, map: &'a mut HashMap<Ident, PathBuf>) -> Self {
        MacroFinder { file, map }
    }
}
impl<'ast> Visit<'ast> for MacroFinder<'_> {
    fn visit_item_macro(&mut self, i: &'ast ItemMacro) {
        if i.attrs.iter().any(|a| a.path().is_ident("macro_export")) {
            if let Some(ident) = i.ident.to_owned() {
                self.map.insert(ident, self.file.to_path_buf());
            }
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
