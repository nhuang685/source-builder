use crate::errors::Errors;
use anyhow::{Context, anyhow, ensure};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use syn::fold::Fold;
use syn::spanned::Spanned;
use syn::{Ident, Item, ItemUse, PathSegment, UsePath, UseTree, fold};

/// Whether a `use` group only imports macros, erroring when it mixes macro and
/// non-macro imports (they need different rewrites) or when it is empty.
fn is_macro_group(
    items: &syn::punctuated::Punctuated<UseTree, syn::Token![,]>,
    macro_file_map: &HashMap<Ident, PathBuf>,
) -> anyhow::Result<bool> {
    let mut res = None;
    for item in items {
        let cur = match item {
            UseTree::Name(n) => macro_file_map.contains_key(&n.ident),
            UseTree::Rename(r) => macro_file_map.contains_key(&r.ident),
            _ => false,
        };
        ensure!(
            res.is_none_or(|res| res == cur),
            "can't have both macro and path in use statement"
        );
        res = Some(cur);
    }
    res.ok_or_else(|| anyhow!("empty use group is not supported"))
}

struct SourceVisitor<'a> {
    library_name: &'a str,
    macro_file_map: &'a HashMap<Ident, PathBuf>,
    errors: Errors,
}

impl SourceVisitor<'_> {
    fn is_macro_item_use(&self, i: &ItemUse) -> anyhow::Result<bool> {
        if let UseTree::Path(p) = &i.tree
            && p.ident == self.library_name
        {
            match &*p.tree {
                UseTree::Name(n) => Ok(self.macro_file_map.contains_key(&n.ident)),
                UseTree::Rename(r) => Ok(self.macro_file_map.contains_key(&r.ident)),
                UseTree::Group(g) => is_macro_group(&g.items, self.macro_file_map),
                _ => Ok(false),
            }
        } else {
            Ok(false)
        }
    }
    /// Same as [`Self::is_macro_item_use`], recording the error for later
    /// propagation since `Fold` cannot return one.
    fn is_macro_item_use_or_record(&mut self, i: &ItemUse) -> bool {
        match self.is_macro_item_use(i) {
            Ok(res) => res,
            Err(err) => {
                self.errors.push(err);
                false
            }
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
        if self.is_macro_item_use_or_record(&i)
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
        let Some(first) = i.segments.first() else {
            return i;
        };
        if self.is_macro_path(&i) {
            let sp = first.span();
            if let Some(first) = i.segments.first_mut() {
                first.ident = Ident::new("crate", sp);
            }
        } else if first.ident == self.library_name {
            let sp = first.ident.span();
            i.segments.insert(
                0,
                PathSegment {
                    ident: Ident::new("crate", sp),
                    arguments: syn::PathArguments::None,
                },
            );
        }
        fold::fold_path(self, i)
    }
    fn fold_file(&mut self, mut i: syn::File) -> syn::File {
        let items = std::mem::take(&mut i.items);
        i.items = items
            .into_iter()
            .filter(|item| match item {
                Item::Use(item) => !self.is_macro_item_use_or_record(item),
                _ => true,
            })
            .collect();
        fold::fold_file(self, i)
    }
}

struct LibraryVisitor<'a> {
    library_name: &'a str,
    macro_file_map: &'a HashMap<Ident, PathBuf>,
    errors: Errors,
}

impl LibraryVisitor<'_> {
    fn is_macro_item_use(&self, i: &ItemUse) -> anyhow::Result<bool> {
        if let UseTree::Path(p) = &i.tree
            && p.ident == "crate"
        {
            match &*p.tree {
                UseTree::Name(n) => Ok(self.macro_file_map.contains_key(&n.ident)),
                UseTree::Rename(r) => Ok(self.macro_file_map.contains_key(&r.ident)),
                UseTree::Group(g) => is_macro_group(&g.items, self.macro_file_map),
                _ => Ok(false),
            }
        } else {
            Ok(false)
        }
    }
    /// Same as [`Self::is_macro_item_use`], recording the error for later
    /// propagation since `Fold` cannot return one.
    fn is_macro_item_use_or_record(&mut self, i: &ItemUse) -> bool {
        match self.is_macro_item_use(i) {
            Ok(res) => res,
            Err(err) => {
                self.errors.push(err);
                false
            }
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
        if !self.is_macro_item_use_or_record(&i)
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
        let Some(first) = i.segments.first() else {
            return i;
        };
        if !self.is_macro_path(&i) && first.ident == "crate" {
            let sp = first.ident.span();
            i.segments.insert(
                1,
                PathSegment {
                    ident: Ident::new(self.library_name, sp),
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
            match syn::parse_str(&result) {
                Ok(tokens) => i.mac.tokens = tokens,
                Err(err) => self.errors.push(anyhow::Error::new(err).context(format!(
                    "failed to reparse rewritten body of macro `{}`",
                    i.ident.as_ref().map(Ident::to_string).unwrap_or_default()
                ))),
            }
        }
        fold::fold_item_macro(self, i)
    }
}

fn get_module_path(file: &Path, library_path: &Path) -> anyhow::Result<Vec<String>> {
    let name = file
        .file_name()
        .ok_or_else(|| anyhow!("`{}` is not a file path", file.display()))?;
    if name == "lib.rs" {
        return Ok(Vec::new());
    }
    // assume library file
    let file = file.strip_prefix(library_path).with_context(|| {
        format!(
            "`{}` is not inside library path `{}`",
            file.display(),
            library_path.display()
        )
    })?;
    let mut file = file.to_path_buf();
    file.set_extension("");
    if file.file_name().is_some_and(|name| name == "mod") {
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
    let mut file = parse_file(source)?;
    let mut visitor = SourceVisitor {
        library_name,
        macro_file_map,
        errors: Errors::default(),
    };
    file = visitor.fold_file(file);
    visitor
        .errors
        .into_result()
        .with_context(|| format!("failed to rewrite source `{}`", source.display()))?;
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
        let mut src = parse_file(&file)?;
        let mut visitor = LibraryVisitor {
            library_name,
            macro_file_map,
            errors: Errors::default(),
        };
        src = visitor.fold_file(src);
        visitor
            .errors
            .into_result()
            .with_context(|| format!("failed to rewrite library file `{}`", file.display()))?;
        content += &prettyplease::unparse(&src);
        pre = mods;
    }

    for _ in 0..pre.len() {
        content += "}";
    }
    content += "}";

    let parsed = syn::parse_file(&content)
        .context("failed to parse the generated output, this is a bug in source-builder")?;
    Ok(prettyplease::unparse(&parsed))
}

fn parse_file(path: &Path) -> anyhow::Result<syn::File> {
    let contents =
        fs::read_to_string(path).with_context(|| format!("failed to read `{}`", path.display()))?;
    syn::parse_file(&contents).with_context(|| format!("failed to parse `{}`", path.display()))
}
