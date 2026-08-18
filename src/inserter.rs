use std::fs;
use std::path::{Path, PathBuf};
use syn::fold::Fold;
use syn::{Ident, Item, ItemUse, PathSegment, UsePath, UseTree, fold};

struct SourceVisitor<'a> {
    library_name: &'a str,
}

impl Fold for SourceVisitor<'_> {
    fn fold_item_use(&mut self, mut i: ItemUse) -> ItemUse {
        if let UseTree::Path(p) = i.tree {
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
        if i.segments.first().unwrap().ident == self.library_name {
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
}

struct LibraryVisitor<'a> {
    library_name: &'a str,
}

impl Fold for LibraryVisitor<'_> {
    fn fold_item_use(&mut self, mut i: ItemUse) -> ItemUse {
        if let UseTree::Path(mut p) = i.tree {
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
        if i.segments.first().unwrap().ident == "crate" {
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
) -> anyhow::Result<String> {
    let mut content = String::new();

    // insert source
    let mut file = syn::parse_file(&fs::read_to_string(source)?)?;
    file = SourceVisitor { library_name }.fold_file(file);
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
        src = LibraryVisitor { library_name }.fold_file(src);
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
