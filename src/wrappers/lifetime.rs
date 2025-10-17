use core::marker::PhantomData;

pub struct Lifetime<'a, T> {
    _value: T,
    _marker: PhantomData<&'a T>,
}

impl<T> Lifetime<'_, T> {
    pub fn new(value: T) -> Self {
        Self {
            _value: value,
            _marker: PhantomData,
        }
    }

    /// # Safety
    /// Bringing the inner value out of the lifetime wrapper may violate the lifetime contract.
    pub unsafe fn into_inner(self) -> T {
        self._value
    }
}
