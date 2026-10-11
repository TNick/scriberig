//! Defines immutable ordered collections used by canonical semantic state.

/// An immutable ordered sequence whose child identity is independent of position.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Collection<T> {
    values: Vec<T>,
}

impl<T> Collection<T> {
    ///
    /// # Returns
    /// The requested operation result.
    /// Creates an empty collection.
    #[must_use]
    pub const fn create() -> Self {
        Self { values: Vec::new() }
    }

    ///
    /// # Returns
    /// The requested operation result.
    /// Returns the number of values.
    #[must_use]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    ///
    /// # Returns
    /// The requested operation result.
    /// Returns whether the collection has no values.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// # Parameters
    /// - `index`: Relevant operation input.
    ///
    /// # Returns
    /// The requested operation result.
    /// Returns a value by position.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&T> {
        self.values.get(index)
    }

    ///
    /// # Returns
    /// The requested operation result.
    /// Iterates values in their canonical order.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.values.iter()
    }

    /// # Parameters
    /// - `index`: Relevant operation input.
    /// - `value`: Relevant operation input.
    ///
    /// # Returns
    /// The requested operation result.
    /// Inserts a value at a position.
    #[must_use]
    pub fn insert(&self, index: usize, value: T) -> Self
    where
        T: Clone,
    {
        let mut values = self.values.clone();
        values.insert(index.min(values.len()), value);
        Self { values }
    }

    /// # Parameters
    /// - `index`: Relevant operation input.
    ///
    /// # Returns
    /// The requested operation result.
    /// Removes a value at a position.
    #[must_use]
    pub fn remove(&self, index: usize) -> Self
    where
        T: Clone,
    {
        let mut values = self.values.clone();
        if index < values.len() {
            values.remove(index);
        }
        Self { values }
    }

    /// # Parameters
    /// - `values`: Relevant operation input.
    ///
    /// # Returns
    /// The requested operation result.
    /// Replaces the sequence with a new ordered sequence.
    #[must_use]
    pub fn replace(&self, values: impl IntoIterator<Item = T>) -> Self {
        Self {
            values: values.into_iter().collect(),
        }
    }
}
