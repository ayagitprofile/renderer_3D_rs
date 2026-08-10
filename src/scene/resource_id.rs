#[derive(Debug, Eq)]
pub struct ResourceID<T> {
    pub index: usize,
    _marker: std::marker::PhantomData<fn() -> T>,
}

impl<T> PartialEq for ResourceID<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self._marker == other._marker
    }
}

impl<T> Copy for ResourceID<T> {}

impl<T> Clone for ResourceID<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> ResourceID<T> {
    pub const fn new(index: usize) -> Self {
        Self {
            index,
            _marker: std::marker::PhantomData,
        }
    }
}
