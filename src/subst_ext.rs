use std::borrow::Cow;

pub use subst::{substitute, VariableMap};

pub struct LocalVec<'a, T>(&'a Vec<T>);

impl<'a, T> LocalVec<'a, T> {
    pub fn new(vec: &'a Vec<T>) -> Self {
        Self(vec)
    }

    pub fn as_slice(&self) -> &[T] {
        self.0.as_slice()
    }
}

impl<'a, T: AsRef<str>> VariableMap<'a> for LocalVec<'a, T> {
    type Value = Cow<'a, str>;

    #[inline]
    fn get(&'a self, key: &str) -> Option<Self::Value> {
        str::parse::<usize>(key)
            .ok()
            .filter(|index| *index > 0)
            .filter(|index| *index <= self.0.len())
            .map(|index| &self.as_slice()[index - 1])
            .map(|s| Cow::from(s.as_ref()))
    }
}

#[derive(Debug)]
pub struct IgnoreMissing<'a, V, T: VariableMap<'a, Value = V>> {
    inner: &'a T,
}

impl<'a, V, T: VariableMap<'a, Value = V>> IgnoreMissing<'a, V, T> {
    pub fn new(inner: &'a T) -> Self {
        Self { inner }
    }
}

impl<'a, V, T: VariableMap<'a, Value = V>> VariableMap<'a> for IgnoreMissing<'a, V, T>
where
    V: AsRef<str> + Default,
{
    type Value = V;

    fn get(&'a self, key: &str) -> Option<Self::Value> {
        self.inner.get(key).or_else(|| Some(V::default()))
    }
}
