use std::fmt::Debug;

use indexmap::map::IndexMap;

use crate::front::ast;
use crate::front::error::{Error, Result};

#[derive(Debug)]
pub struct Scope<'p, V> {
    parent: Option<&'p Scope<'p, V>>,
    values: IndexMap<String, V>,
}

impl<V: Debug> Scope<'_, V> {
    pub fn nest(&self) -> Scope<V> {
        Scope { parent: Some(self), values: Default::default() }
    }

    pub fn declare<'a>(&mut self, id: &'a ast::Identifier, var: V) -> Result<'a, ()> {
        if self.values.insert(id.string.to_owned(), var).is_some() {
            Err(Error::IdentifierDeclaredTwice(id))
        } else {
            Ok(())
        }
    }

    pub fn maybe_declare<'a>(&mut self, id: &'a ast::MaybeIdentifier, var: V) -> Result<'a, ()> {
        match id {
            ast::MaybeIdentifier::Identifier(id) =>
                self.declare(id, var),
            ast::MaybeIdentifier::Placeholder(_) =>
                Ok(())
        }
    }

    /// Declare a value with the given id. Panics if the id already exists in this scope.
    pub fn declare_str(&mut self, id: &str, var: V) {
        let prev = self.values.insert(id.to_owned(), var);

        if let Some(prev) = prev {
            panic!("Id '{}' already exists in this scope with value {:?}", id, prev)
        }
    }

    /// Find the given identifier in this scope.
    /// Walks up into the parent scopes until a scope without a parent is found,
    /// then looks in the `root` scope. If no value is found returns `Err`.
    pub fn find<'a, 's>(&'s self, root: Option<&'s Self>, id: &'a ast::Identifier) -> Result<'a, &V> {
        if let Some(s) = self.values.get(&id.string) {
            Ok(s)
        } else if let Some(p) = self.parent {
            p.find(root, id)
        } else if let Some(root) = root {
            root.find(None, id)
        } else {
            Err(Error::UndeclaredIdentifier(id))
        }
    }

    /// Find the given identifier in this scope without looking at the parent scope.
    pub fn find_immediate_str(&self, id: &str) -> Option<&V> {
        self.values.get(id)
    }

    /// The amount of values declared in this scope without taking the parent scope into account.
    pub fn size(&self) -> usize {
        self.values.len()
    }
}

impl<'p, V> Default for Scope<'p, V> {
    fn default() -> Self {
        Self {
            parent: Default::default(),
            values: Default::default(),
        }
    }
}
