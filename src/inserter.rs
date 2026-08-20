use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use syn::fold::Fold;
use syn::spanned::Spanned;
use syn::{Ident, Item, ItemUse, PathSegment, UsePath, UseTree, fold};

struct SourceVisitor<'a> {
    library_name: &'a str,
    macro_file_map: &'a HashMap<Ident, PathBuf>,
}

impl SourceVisitor<'_> {
    fn is_macro_item_use(&self, i: &ItemUse) -> bool {
        if let UseTree::Path(p) = &i.tree
            && p.ident == self.library_name
        {
            match &*p.tree {
                UseTree::Name(n) => self.macro_file_map.contains_key(&n.ident),
                UseTree::Rename(r) => self.macro_file_map.contains_key(&r.ident),
                UseTree::Group(g) => {
                    let mut res = None;
                    for item in &g.items {
                        let cur = match item {
                            UseTree::Name(n) => self.macro_file_map.contains_key(&n.ident),
                            UseTree::Rename(r) => self.macro_file_map.contains_key(&r.ident),
                            _ => false,
                        };
                        assert!(
                            res.is_none_or(|res| res == cur),
                            "Can't have both macro and path in use statement"
                        );
                        res = Some(cur);
                    }
                    res.unwrap()
                }
                _ => false,
            }
        } else {
            false
        }
    }
    fn is_macro_path(&self, i: &syn::Path) -> bool {
        // must be of form library_name::macro
        i.segments.len() == 2
            && i.segments.first().unwrap().ident == self.library_name
            && self
                .macro_file_map
                .contains_key(&i.segments.last().unwrap().ident)
    }
}

impl Fold for SourceVisitor<'_> {
    fn fold_item_use(&mut self, mut i: ItemUse) -> ItemUse {
        if self.is_macro_item_use(&i)
            && let UseTree::Path(mut p) = i.tree
        {
            p.ident = Ident::new("crate", p.ident.span());
            i.tree = UseTree::Path(p);
        } else if let UseTree::Path(p) = i.tree {
            i.tree = if p.ident == self.library_name {
                let np = UsePath {
                    ident: Ident::new("crate", p.ident.span()),
                    colon2_token: p.colon2_token,
                    tree: Box::new(UseTree::Path(p)),
                };
                UseTree::Path(np)
            } else {
                UseTree::Path(p)
            }
        }
        fold::fold_item_use(self, i)
    }
    fn fold_path(&mut self, mut i: syn::Path) -> syn::Path {
        if self.is_macro_path(&i) {
            let sp = i.segments.first().unwrap().span();
            i.segments.first_mut().unwrap().ident = Ident::new("crate", sp);
        } else if i.segments.first().unwrap().ident == self.library_name {
            i.segments.insert(
                0,
                PathSegment {
                    ident: Ident::new("crate", i.segments[0].ident.span()),
                    arguments: syn::PathArguments::None,
                },
            );
        }
        fold::fold_path(self, i)
    }
    fn fold_file(&mut self, mut i: syn::File) -> syn::File {
        i.items.retain(|item| match item {
            Item::Use(item) => !self.is_macro_item_use(item),
            _ => true,
        });
        fold::fold_file(self, i)
    }
}

struct LibraryVisitor<'a> {
    library_name: &'a str,
    macro_file_map: &'a HashMap<Ident, PathBuf>,
}

impl LibraryVisitor<'_> {
    fn is_macro_item_use(&self, i: &ItemUse) -> bool {
        if let UseTree::Path(p) = &i.tree
            && p.ident == "crate"
        {
            match &*p.tree {
                UseTree::Name(n) => self.macro_file_map.contains_key(&n.ident),
                UseTree::Rename(r) => self.macro_file_map.contains_key(&r.ident),
                UseTree::Group(g) => {
                    let mut res = None;
                    for item in &g.items {
                        let cur = match item {
                            UseTree::Name(n) => self.macro_file_map.contains_key(&n.ident),
                            UseTree::Rename(r) => self.macro_file_map.contains_key(&r.ident),
                            _ => false,
                        };
                        assert!(
                            res.is_none_or(|res| res == cur),
                            "Can't have both macro and path in use statement"
                        );
                        res = Some(cur);
                    }
                    res.unwrap()
                }
                _ => false,
            }
        } else {
            false
        }
    }
    fn is_macro_path(&self, i: &syn::Path) -> bool {
        // must be of form library_name::macro
        i.segments.len() == 2
            && i.segments.first().unwrap().ident == "crate"
            && self
                .macro_file_map
                .contains_key(&i.segments.last().unwrap().ident)
    }
}

impl Fold for LibraryVisitor<'_> {
    fn fold_item_use(&mut self, mut i: ItemUse) -> ItemUse {
        if !self.is_macro_item_use(&i)
            && let UseTree::Path(mut p) = i.tree
        {
            i.tree = if p.ident == "crate" {
                p.ident = Ident::new(self.library_name, p.ident.span());
                let np = UsePath {
                    ident: Ident::new("crate", p.ident.span()),
                    colon2_token: p.colon2_token,
                    tree: Box::new(UseTree::Path(p)),
                };
                UseTree::Path(np)
            } else {
                UseTree::Path(p)
            }
        }
        fold::fold_item_use(self, i)
    }

    fn fold_path(&mut self, mut i: syn::Path) -> syn::Path {
        if !self.is_macro_path(&i) && i.segments.first().unwrap().ident == "crate" {
            i.segments.insert(
                1,
                PathSegment {
                    ident: Ident::new(self.library_name, i.segments[0].ident.span()),
                    arguments: syn::PathArguments::None,
                },
            );
        }
        fold::fold_path(self, i)
    }

    fn fold_file(&mut self, mut i: syn::File) -> syn::File {
        i.items.retain(|item| match item {
            Item::Mod(m) => m.content.is_some(),
            _ => true,
        });
        fold::fold_file(self, i)
    }

    fn fold_item_macro(&mut self, mut i: syn::ItemMacro) -> syn::ItemMacro {
        // taken from visit_item_macro_mut in https://github.com/rust-competitive-helper/rust-competitive-helper/blob/main/rust-competitive-helper-util/src/new_build.rs
        if i.ident.is_some() {
            let body = i.mac.tokens.to_string();
            let mut state = 0;
            let mut result = String::new();
            let mut ident = String::new();
            for token in body.split(' ') {
                if state == 0 && token == "$" {
                    state = 1;
                } else if state == 1 && token == "crate" {
                    state = 2;
                } else if state == 2 && token == "::" {
                    state = 3;
                } else if state == 3 {
                    ident = token.to_string();
                    state = 4;
                    continue;
                } else if state == 4 {
                    if token == "!" {
                        result += &ident;
                        result += " ";
                    } else {
                        result += self.library_name;
                        result += "::";
                        result += &ident;
                        result += " ";
                    }
                    state = 0;
                } else {
                    state = 0;
                }
                result += token;
                result += " ";
            }
            if state == 4 {
                result += self.library_name;
                result += "::";
                result += &ident;
            }
            i.mac.tokens = syn::parse_str(&result).unwrap();
        }
        fold::fold_item_macro(self, i)
    }
}

fn get_module_path(file: &Path, library_path: &Path) -> anyhow::Result<Vec<String>> {
    if file.file_name().unwrap() == "lib.rs" {
        return Ok(Vec::new());
    }
    // assume library file
    let mut file = file.strip_prefix(library_path)?.to_path_buf();
    file.set_extension("");
    if file.file_name().unwrap() == "mod" {
        file.pop();
    }
    Ok(file
        .iter()
        .map(|s| s.to_string_lossy().into_owned())
        .collect::<Vec<_>>())
}
pub fn gen_file(
    source: &Path,
    library_name: &str,
    library_path: &Path,
    files: Vec<PathBuf>,
    macro_file_map: &HashMap<Ident, PathBuf>,
) -> anyhow::Result<String> {
    let mut content = String::new();

    // insert source
    let mut file = syn::parse_file(&fs::read_to_string(source)?)?;
    file = SourceVisitor {
        library_name,
        macro_file_map,
    }
    .fold_file(file);
    content += &prettyplease::unparse(&file);

    // insert library files
    content += &format!("pub mod {library_name} {{");
    let mut pre = Vec::new();
    for file in files {
        let mods = get_module_path(&file, library_path)?;
        let cut = pre
            .iter()
            .zip(&mods)
            .enumerate()
            .find_map(|(i, (a, b))| (a != b).then_some(i))
            .unwrap_or(pre.len().min(mods.len()));
        for _ in cut..pre.len() {
            content += "}";
        }
        for module in &mods[cut..] {
            content += &format!("pub mod {module} {{");
        }
        let mut src = syn::parse_file(&fs::read_to_string(file)?)?;
        src = LibraryVisitor {
            library_name,
            macro_file_map,
        }
        .fold_file(src);
        content += &prettyplease::unparse(&src);
        pre = mods;
    }

    for _ in 0..pre.len() {
        content += "}";
    }
    content += "}";

    content = prettyplease::unparse(&syn::parse_file(&content)?);

    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn ident(name: &str) -> Ident {
        syn::parse_str(name).unwrap()
    }

    fn macro_map(names: &[&str]) -> HashMap<Ident, PathBuf> {
        names
            .iter()
            .map(|n| (ident(n), PathBuf::from("macros.rs")))
            .collect()
    }

    fn fold_source(contents: &str, macros: &[&str]) -> String {
        let map = macro_map(macros);
        let file = syn::parse_file(contents).unwrap();
        let folded = SourceVisitor {
            library_name: "algo_lib",
            macro_file_map: &map,
        }
        .fold_file(file);
        prettyplease::unparse(&folded)
    }

    fn fold_library(contents: &str, macros: &[&str]) -> String {
        let map = macro_map(macros);
        let file = syn::parse_file(contents).unwrap();
        let folded = LibraryVisitor {
            library_name: "algo_lib",
            macro_file_map: &map,
        }
        .fold_file(file);
        prettyplease::unparse(&folded)
    }

    #[test]
    fn source_use_of_library_is_nested_under_crate() {
        assert_eq!(
            fold_source("use algo_lib::a::Foo;\n", &[]),
            "use crate::algo_lib::a::Foo;\n"
        );
    }

    #[test]
    fn source_use_of_other_crate_is_untouched() {
        assert_eq!(
            fold_source("use std::io::Write;\n", &[]),
            "use std::io::Write;\n"
        );
        assert_eq!(fold_source("use std::io;\n", &[]), "use std::io;\n");
    }

    #[test]
    fn source_macro_imports_are_removed() {
        assert_eq!(
            fold_source("use algo_lib::shout;\nuse algo_lib::a::Foo;\n", &["shout"]),
            "use crate::algo_lib::a::Foo;\n"
        );
        assert_eq!(
            fold_source("use algo_lib::shout as yell;\n", &["shout"]),
            ""
        );
        assert_eq!(
            fold_source("use algo_lib::{shout, whisper};\n", &["shout", "whisper"]),
            ""
        );
    }

    #[test]
    fn source_macro_import_is_rewritten_to_crate_root() {
        let map = macro_map(&["shout"]);
        let item = syn::parse_str("use algo_lib::shout;").unwrap();
        let folded = SourceVisitor {
            library_name: "algo_lib",
            macro_file_map: &map,
        }
        .fold_item_use(item);

        let mut file = syn::parse_file("").unwrap();
        file.items = vec![Item::Use(folded)];
        assert_eq!(prettyplease::unparse(&file), "use crate::shout;\n");
    }

    #[test]
    fn source_renamed_macro_group_is_removed() {
        assert_eq!(
            fold_source(
                "use algo_lib::{shout as s, whisper as w};\n",
                &["shout", "whisper"]
            ),
            ""
        );
    }

    #[test]
    fn source_grouped_module_imports_are_nested() {
        assert_eq!(
            fold_source("use algo_lib::{a::Foo, b};\n", &["shout"]),
            "use crate::algo_lib::{a::Foo, b};\n"
        );
    }

    #[test]
    #[should_panic(expected = "Can't have both macro and path in use statement")]
    fn source_rejects_mixed_macro_and_module_group() {
        fold_source("use algo_lib::{shout, a};\n", &["shout"]);
    }

    #[test]
    fn source_paths_are_rewritten() {
        assert_eq!(
            fold_source("fn main() {\n    algo_lib::a::f();\n}\n", &[]),
            "fn main() {\n    crate::algo_lib::a::f();\n}\n"
        );
    }

    #[test]
    fn source_macro_paths_point_at_crate_root() {
        assert_eq!(
            fold_source("fn main() {\n    algo_lib::shout!();\n}\n", &["shout"]),
            "fn main() {\n    crate::shout!();\n}\n"
        );
    }

    #[test]
    fn library_crate_paths_gain_library_module() {
        assert_eq!(
            fold_library("use crate::b::Bar;\n", &[]),
            "use crate::algo_lib::b::Bar;\n"
        );
        assert_eq!(
            fold_library("fn f() {\n    crate::b::g();\n}\n", &[]),
            "fn f() {\n    crate::algo_lib::b::g();\n}\n"
        );
    }

    #[test]
    fn library_macro_references_stay_at_crate_root() {
        assert_eq!(
            fold_library("use crate::shout;\n", &["shout"]),
            "use crate::shout;\n"
        );
        assert_eq!(
            fold_library("fn f() {\n    crate::shout!();\n}\n", &["shout"]),
            "fn f() {\n    crate::shout!();\n}\n"
        );
        assert_eq!(
            fold_library("use crate::{shout, whisper};\n", &["shout", "whisper"]),
            "use crate::{shout, whisper};\n"
        );
        assert_eq!(
            fold_library("use crate::shout as yell;\n", &["shout"]),
            "use crate::shout as yell;\n"
        );
    }

    #[test]
    fn library_renamed_macro_group_stays_at_crate_root() {
        assert_eq!(
            fold_library(
                "use crate::{shout as s, whisper as w};\n",
                &["shout", "whisper"]
            ),
            "use crate::{shout as s, whisper as w};\n"
        );
    }

    #[test]
    fn library_glob_group_is_not_treated_as_macro() {
        assert_eq!(
            fold_library("use crate::a::*;\n", &["shout"]),
            "use crate::algo_lib::a::*;\n"
        );
    }

    #[test]
    fn library_non_crate_uses_are_untouched() {
        assert_eq!(
            fold_library("use std::io::Write;\n", &[]),
            "use std::io::Write;\n"
        );
    }

    #[test]
    fn library_mod_declarations_are_dropped() {
        assert_eq!(
            fold_library("pub mod a;\npub mod b {}\n", &[]),
            "pub mod b {}\n"
        );
    }

    #[test]
    fn library_macro_bodies_get_library_module_inserted() {
        let out = fold_library(
            "#[macro_export]\nmacro_rules! shout {\n    () => { $crate::util::helper() };\n}\n",
            &["shout"],
        );
        assert!(out.contains("$crate::algo_lib::util::helper"), "{out}");
    }

    #[test]
    fn library_macro_bodies_keep_macro_invocations_at_root() {
        let out = fold_library(
            "#[macro_export]\nmacro_rules! shout {\n    () => { $crate::whisper!() };\n}\n",
            &["shout", "whisper"],
        );
        assert!(out.contains("$crate::whisper!"), "{out}");
        assert!(!out.contains("algo_lib"), "{out}");
    }

    #[test]
    fn library_macro_bodies_handle_trailing_crate_reference() {
        let out = fold_library(
            "#[macro_export]\nmacro_rules! shout {\n    () => { $crate::VALUE };\n}\n",
            &["shout"],
        );
        assert!(out.contains("$crate::algo_lib::VALUE"), "{out}");

        // token stream ending on a `$crate::ident` reference
        let map = macro_map(&["shout"]);
        let item = syn::parse_str("macro_rules! shout { $crate::VALUE }").unwrap();
        let folded = LibraryVisitor {
            library_name: "algo_lib",
            macro_file_map: &map,
        }
        .fold_item_macro(item);
        assert_eq!(
            folded.mac.tokens.to_string(),
            "$ crate :: algo_lib :: VALUE"
        );
    }

    #[test]
    fn module_path_of_lib_rs_is_empty() {
        let library_path = Path::new("/lib/src");
        assert!(
            get_module_path(&library_path.join("lib.rs"), library_path)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn module_path_uses_file_and_directory_names() {
        let library_path = Path::new("/lib/src");
        assert_eq!(
            get_module_path(&library_path.join("a.rs"), library_path).unwrap(),
            vec!["a".to_string()]
        );
        assert_eq!(
            get_module_path(&library_path.join("a/mod.rs"), library_path).unwrap(),
            vec!["a".to_string()]
        );
        assert_eq!(
            get_module_path(&library_path.join("a/b/c.rs"), library_path).unwrap(),
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn module_path_errors_outside_library() {
        assert!(get_module_path(Path::new("/other/a.rs"), Path::new("/lib/src")).is_err());
    }

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
            self.dir.path().join("src")
        }

        fn write_source(&self, contents: &str) -> PathBuf {
            let path = self.dir.path().join("source.rs");
            fs::write(&path, contents).unwrap();
            path
        }

        fn write_lib(&self, name: &str, contents: &str) -> PathBuf {
            let path = self.lib_path().join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, contents).unwrap();
            path
        }
    }

    #[test]
    fn gen_file_nests_library_modules() {
        let fx = Fixture::new();
        let source = fx.write_source("use algo_lib::a::b::Foo;\nfn main() {}\n");
        let lib = fx.write_lib("lib.rs", "pub mod a;\n");
        let a = fx.write_lib("a/mod.rs", "pub mod b;\n");
        let b = fx.write_lib("a/b.rs", "pub struct Foo;\n");
        let other = fx.write_lib("other.rs", "pub struct Other;\n");

        let content = gen_file(
            &source,
            "algo_lib",
            &fx.lib_path(),
            vec![lib, a, b, other],
            &HashMap::new(),
        )
        .unwrap();

        assert_eq!(
            content,
            "use crate::algo_lib::a::b::Foo;\nfn main() {}\npub mod algo_lib {\n    pub mod a {\n        pub mod b {\n            pub struct Foo;\n        }\n    }\n    pub mod other {\n        pub struct Other;\n    }\n}\n"
        );
    }

    #[test]
    fn gen_file_rewrites_macro_references() {
        let fx = Fixture::new();
        let source = fx.write_source("use algo_lib::shout;\nfn main() {\n    shout!();\n}\n");
        let lib = fx.write_lib("lib.rs", "pub mod macros;\n");
        let macros = fx.write_lib(
            "macros.rs",
            "#[macro_export]\nmacro_rules! shout {\n    () => { $crate::macros::helper() };\n}\npub fn helper() {}\n",
        );
        let macro_file_map = HashMap::from([(ident("shout"), macros.clone())]);

        let content = gen_file(
            &source,
            "algo_lib",
            &fx.lib_path(),
            vec![lib, macros],
            &macro_file_map,
        )
        .unwrap();

        assert!(!content.contains("use algo_lib::shout"), "{content}");
        assert!(!content.contains("use crate::shout"), "{content}");
        assert!(
            content.contains("$crate::algo_lib::macros::helper"),
            "{content}"
        );
    }

    #[test]
    fn gen_file_errors_on_missing_source() {
        let fx = Fixture::new();
        let lib = fx.write_lib("lib.rs", "");

        assert!(
            gen_file(
                &fx.dir.path().join("missing.rs"),
                "algo_lib",
                &fx.lib_path(),
                vec![lib],
                &HashMap::new(),
            )
            .is_err()
        );
    }
}
