//! Bundles.
//!
//! When an expression is evaluated, it becomes a bundle which may be pointed at other nodes.
//!
//! Bundles exist as parameters to node construction, but are actually deconstituted to raw graph edges to account for
//! things like sending two bundles to the same place, then reconstituted on an as-needed basis to provide nodes with
//! views of their incoming data.
use std::collections::HashMap;

/// A bundle.
///
/// Bundles consist of an array part, usually containing channels of audio, and a kv part, usually containing parameters
/// like frequency.
#[derive(Debug)]
pub(crate) struct Bundle<T> {
    /// The positional part of the bundle.
    array: Vec<T>,

    /// The key-value part of the bundle, usually things like `frequency`.
    kv: HashMap<String, T>,
}

impl<T> Bundle<T> {
    pub(crate) fn new() -> Bundle<T> {
        Bundle {
            array: vec![],
            kv: HashMap::new(),
        }
    }

    pub(crate) fn push_array(&mut self, val: T) {
        self.array.push(val);
    }

    pub(crate) fn set_key(&mut self, key: &str, val: T) -> Option<T> {
        // We take &str so that we can e.g. optimize with hashbrown later if we have to, but let's not bother for now.
        self.kv.insert(key.to_string(), val)
    }

    pub(crate) fn iter_array(&self) -> impl Iterator<Item = &T> {
        self.array.iter()
    }

    pub fn iter_kv(&self) -> impl Iterator<Item = (&str, &T)> {
        self.kv.iter().map(|x| (x.0.as_str(), x.1))
    }
}

impl<T> Default for Bundle<T> {
    fn default() -> Self {
        Self::new()
    }
}
