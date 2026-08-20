use crate::utils::parse_file;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path;
use std::path::PathBuf;
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
        vis.visit_file(&parse_file(&file)?);
    }
    vis.files.remove(source);
    vis.files.remove(&library_path.join("lib.rs"));
    Ok(vis.files)
}
