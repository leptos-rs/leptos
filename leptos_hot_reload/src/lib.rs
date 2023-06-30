extern crate proc_macro;

use anyhow::Result;
use camino::Utf8PathBuf;
use diff::Patches;
use node::LNode;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
};
use syn::{
    spanned::Spanned,
    visit::{self, Visit},
    Macro,
};
use walkdir::WalkDir;

pub mod diff;
pub mod node;
pub mod parsing;

pub const HOT_RELOAD_JS: &str = include_str!("patch.js");

#[derive(Debug, Clone, Default)]
pub struct ViewMacros {
    // keyed by original location identifier
    views: Arc<RwLock<HashMap<Utf8PathBuf, Vec<MacroInvocation>>>>,
}

impl ViewMacros {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_from_paths<T: AsRef<Path>>(&self, paths: &[T]) -> Result<()> {
        let mut views = HashMap::new();

        for path in paths {
            for entry in WalkDir::new(path).into_iter().flatten() {
                if entry.file_type().is_file() {
                    let path: PathBuf = entry.path().into();
                    let path = Utf8PathBuf::try_from(path)?;
                    if path.extension() == Some("rs") || path.ends_with(".rs") {
                        let macros = Self::parse_file(&path)?;
                        let entry = views.entry(path.clone()).or_default();
                        *entry = macros;
                    }
                }
            }
        }

        *self.views.write() = views;

        Ok(())
    }

    pub fn parse_file(path: &Utf8PathBuf) -> Result<Vec<MacroInvocation>> {
        let mut file = File::open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        let ast = syn::parse_file(&content)?;

        let mut visitor = ViewMacroVisitor::default();
        visitor.visit_file(&ast);
        let mut views = Vec::new();
        for view in visitor.views {
            let span = view.span();
            let id = span_to_stable_id(path, span.start().line);
            let mut tokens = view.tokens.clone().into_iter();
            tokens.next(); // cx
            tokens.next(); // ,
                           // TODO handle class = ...
            let rsx =
                rstml::parse2(tokens.collect::<proc_macro2::TokenStream>())?;
            let template = LNode::parse_view(rsx)?;
            views.push(MacroInvocation { id, template })
        }
        Ok(views)
    }

    pub fn patch(&self, path: &Utf8PathBuf) -> Result<Option<Patches>> {
        let new_views = Self::parse_file(path)?;
        let mut lock = self.views.write();
        let diffs = match lock.get(path) {
            None => return Ok(None),
            Some(current_views) => {
                if current_views.len() == new_views.len() {
                    let mut diffs = Vec::new();
                    for (current_view, new_view) in
                        current_views.iter().zip(&new_views)
                    {
                        if current_view.id == new_view.id
                            && current_view.template != new_view.template
                        {
                            diffs.push((
                                current_view.id.clone(),
                                current_view.template.diff(&new_view.template),
                            ));
                        }
                    }
                    diffs
                } else {
                    return Ok(None);
                }
            }
        };

        // update the status to the new views
        lock.insert(path.clone(), new_views);

        Ok(Some(Patches(diffs)))
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MacroInvocation {
    id: String,
    template: LNode,
}

impl std::fmt::Debug for MacroInvocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MacroInvocation")
            .field("id", &self.id)
            .finish()
    }
}

#[derive(Default, Debug)]
pub struct ViewMacroVisitor<'a> {
    views: Vec<&'a Macro>,
}

impl<'ast> Visit<'ast> for ViewMacroVisitor<'ast> {
    fn visit_macro(&mut self, node: &'ast Macro) {
        let ident = node.path.get_ident().map(|n| n.to_string());
        if ident == Some("view".to_string()) {
            self.views.push(node);
        }

        // Delegate to the default impl to visit any nested functions.
        visit::visit_macro(self, node);
    }
}

pub fn span_to_stable_id(path: impl AsRef<Path>, line: usize) -> String {
    let file = path
        .as_ref()
        .to_str()
        .unwrap_or_default()
        .replace(['/', '\\'], "-");
    format!("{file}-{line}")
}
