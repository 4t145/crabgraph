use std::{collections::HashSet, hash::Hash};

pub trait TryIntoSet<K, A> {
    fn try_into_set(self) -> Result<HashSet<K>, crate::Error>;
}

pub trait IntoSet<K, A> {
    fn into_set(self) -> HashSet<K>;
}
pub enum ByInto {}

impl<T, K> IntoSet<K, ByInto> for T
where
    T: Into<K>,
    K: Hash + Eq,
{
    fn into_set(self) -> HashSet<K> {
        HashSet::from([self.into()])
    }
}
pub enum ByIntoIter {}

impl<K, T: IntoIterator<Item = K>> IntoSet<K, ByIntoIter> for T
where
    K: Hash + Eq,
{
    fn into_set(self) -> HashSet<K> {
        self.into_iter().map(|t| t.into()).collect()
    }
}

pub struct ByResult<T> {
    _marker: std::marker::PhantomData<T>,
}
impl<K, A, T: TryIntoSet<K, A>> TryIntoSet<K, ByResult<A>> for Result<T, crate::Error> {
    fn try_into_set(self) -> Result<HashSet<K>, crate::Error> {
        self.and_then(|s| s.try_into_set())
    }
}
pub struct ByOk<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<K, A, T: IntoSet<K, A>> TryIntoSet<K, ByOk<A>> for T {
    fn try_into_set(self) -> Result<HashSet<K>, crate::Error> {
        Ok(self.into_set())
    }
}

#[macro_export]
macro_rules! map {
    ($($key:expr => $val:expr),*) => {
        std::collections::HashMap::from_iter([
            $(($key, $val)),*
        ])
    };
}