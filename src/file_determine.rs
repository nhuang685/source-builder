use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::{fs, path};
use syn::punctuated::Punctuated;
use syn::visit::Visit;
use syn::{Ident, ItemUse, Path, PathSegment, Token, UseTree, visit};

struct Visitor<'a> {
    pub is_source: bool,
    pub queue: VecDeque<(PathBuf, bool)>,
    pub files: HashSet<PathBuf>,
    pub name_ref: HashMap<Ident, Vec<PathBuf>>,
    pub lib_rt: &'a str,
    pub macro_file_map: &'a HashMap<Ident, PathBuf>,
    pub lib_path: PathBuf,
}

impl Visitor<'_> {
    fn add_file(&mut self, path: PathBuf, is_source: bool) {
        if path.is_file() && !self.files.contains(&path) {
            self.files.insert(path.clone());
            self.queue.push_back((path, is_source));
        }
    }
    fn add_mod(&mut self, path: &path::Path, segment: &str) {
        dbg!(path, segment);
        // two possible:
        // path/segment.rs
        // path/segment/mod.rs
        self.add_file(path.join(format!("{segment}.rs")), false);
        self.add_file(path.join(format!("{segment}/mod.rs")), false);
    }
    fn possible_macro(&mut self, m: &Ident) {
        if let Some(path) = self.macro_file_map.get(m).cloned() {
            self.add_file(path, false);
        }
    }
    fn add_name_ref(&mut self, path: &PathBuf, i: Ident, name: Ident) {
        self.name_ref
            .entry(i)
            .or_default()
            .push(path.join(name.to_string()));
    }
    fn process_use_tree(&mut self, i: &UseTree, mut path: PathBuf) {
        match i {
            UseTree::Path(p) => {
                self.add_name_ref(&path, p.ident.clone(), p.ident.clone());
                let segment = p.ident.to_string();
                self.add_mod(&path, &segment);
                path.push(segment);
                self.process_use_tree(&p.tree, path);
            }
            UseTree::Name(n) => {
                self.add_name_ref(&path, n.ident.clone(), n.ident.clone());
                self.possible_macro(&n.ident);
                let segment = n.ident.to_string();
                self.add_mod(&path, &segment);
            }
            UseTree::Rename(r) => {
                self.add_name_ref(&path, r.rename.clone(), r.ident.clone());
                self.possible_macro(&r.ident);
                let segment = r.ident.to_string();
                self.add_mod(&path, &segment);
            }
            UseTree::Glob(_) => {
                panic!("Can't use glob imports");
            }
            UseTree::Group(g) => {
                for item in &g.items {
                    self.process_use_tree(item, path.clone());
                }
            }
        }
    }
    fn process_path(&mut self, mut path: PathBuf, segments: &Punctuated<PathSegment, Token![::]>) {
        for s in segments.iter().skip(1) {
            let segment = s.ident.to_string();
            self.add_mod(&path, &segment);
            path.push(segment);
        }
    }
}

impl<'ast> Visit<'ast> for Visitor<'_> {
    fn visit_item_use(&mut self, i: &'ast ItemUse) {
        match &i.tree {
            UseTree::Path(p) => {
                if p.ident == self.lib_rt {
                    self.process_use_tree(&p.tree, self.lib_path.clone());
                }
            }
            _ => (),
        };
        visit::visit_item_use(self, i);
    }

    fn visit_path(&mut self, i: &'ast Path) {
        if let Some(PathSegment {
            ident: lead,
            arguments: _,
        }) = i.segments.first()
        {
            // name_ref is not changed during this process, it is specific to use statements
            if lead == self.lib_rt {
                self.process_path(self.lib_path.clone(), &i.segments);
            }
            if let Some(map) = self.name_ref.get(lead).cloned() {
                for pref in map {
                    self.process_path(pref, &i.segments);
                }
            }
        }
        if let Some(PathSegment {
            ident: last,
            arguments: _,
        }) = i.segments.last()
        {
            self.possible_macro(last);
        }
        visit::visit_path(self, i);
    }
}

pub fn determine_files(
    source: &path::Path,
    library_name: &str,
    library_path: &path::Path,
    macro_file_map: &HashMap<Ident, PathBuf>,
) -> anyhow::Result<HashSet<PathBuf>> {
    let mut vis = Visitor {
        is_source: true,
        queue: VecDeque::new(),
        files: HashSet::new(),
        name_ref: HashMap::new(),
        lib_rt: "",
        lib_path: library_path.to_path_buf(),
        macro_file_map,
    };
    vis.add_file(source.to_path_buf(), true);
    vis.add_file(library_path.join("lib.rs"), false);
    while let Some((file, is_source)) = vis.queue.pop_front() {
        if is_source {
            vis.is_source = true;
            vis.lib_rt = library_name;
        } else {
            vis.is_source = false;
            vis.lib_rt = "crate";
        }
        vis.name_ref = HashMap::new();
        vis.visit_file(&syn::parse_file(fs::read_to_string(file)?.as_str())?);
    }
    vis.files.remove(source);
    vis.files.remove(&library_path.join("lib.rs"));
    Ok(vis.files)
}

#[cfg(test)]
mod tests {
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

        assert!(
            determine_files(&fx.source(), "algo_lib", &fx.lib_path(), &HashMap::new()).is_err()
        );
    }

    #[test]
    fn missing_files_are_skipped() {
        let fx = Fixture::new();
        fx.write_lib("lib.rs", "pub mod a;\n");
        fx.write_source("use algo_lib::a::Foo;\nfn main() {}\n");

        assert!(fx.determine(&HashMap::new()).is_empty());
    }
}
