#[derive(Debug, PartialEq, Eq)]
pub struct ResourceID<T> {
    pub index: usize,
    _marker: std::marker::PhantomData<fn() -> T>,
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
