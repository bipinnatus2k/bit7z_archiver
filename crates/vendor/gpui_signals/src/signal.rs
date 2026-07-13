use crate::storage::{with_signal_storage, SignalId};
use gpui::{IntoElement, SharedString};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

pub struct Signal<T> {
    id: SignalId,
    generation: u32,
    _phantom: PhantomData<T>,
}

impl<T> Copy for Signal<T> {}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> PartialEq for Signal<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for Signal<T> {}

impl<T> Hash for Signal<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Signal<bool> {
    pub fn toggle(&self) {
        self.update(|v| *v = !*v);
    }
}

impl<T: fmt::Display + Clone + 'static> IntoElement for Signal<T> {
    type Element = SharedString;

    fn into_element(self) -> Self::Element {
        self.get().to_string().into()
    }
}

impl<T: 'static> Signal<T> {
    pub(crate) fn new(value: T) -> Self {
        with_signal_storage(|storage| {
            let id = storage.insert(value);
            Self {
                id,
                generation: 0,
                _phantom: PhantomData,
            }
        })
    }

    pub fn get(&self) -> T
    where
        T: Clone,
    {
        with_signal_storage(|storage| {
            storage.track_read(self.id);
            storage
                .get::<T>(self.id, self.generation)
                .cloned()
                .expect("Signal value not found")
        })
    }

    pub fn get_untracked(&self) -> T
    where
        T: Clone,
    {
        with_signal_storage(|storage| {
            storage
                .get::<T>(self.id, self.generation)
                .cloned()
                .expect("Signal value not found")
        })
    }

    pub fn set(&self, value: T) {
        if let Some(callbacks) =
            with_signal_storage(|storage| storage.set(self.id, self.generation, value))
        {
            for callback in callbacks {
                callback();
            }
        }
    }

    pub fn set_if_changed(&self, value: T) -> bool
    where
        T: PartialEq,
    {
        let should_update = self.with_untracked(|current| current != &value);
        if should_update {
            self.set(value);
        }
        should_update
    }

    pub fn update(&self, f: impl FnOnce(&mut T)) {
        if let Some((_, callbacks)) =
            with_signal_storage(|storage| storage.update(self.id, self.generation, f))
        {
            for callback in callbacks {
                callback();
            }
        }
    }

    pub fn update_with<R>(&self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        if let Some((result, callbacks)) =
            with_signal_storage(|storage| storage.update(self.id, self.generation, f))
        {
            for callback in callbacks {
                callback();
            }
            return Some(result);
        }
        None
    }

    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        with_signal_storage(|storage| {
            storage.track_read(self.id);
            let value = storage
                .get::<T>(self.id, self.generation)
                .expect("Signal value not found");
            f(value)
        })
    }

    pub fn with_untracked<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        with_signal_storage(|storage| {
            let value = storage
                .get::<T>(self.id, self.generation)
                .expect("Signal value not found");
            f(value)
        })
    }

    pub fn subscribe(&self, callback: impl Fn() + 'static) {
        with_signal_storage(|storage| {
            storage.subscribe(self.id, callback);
        });
    }

    pub fn read_only(self) -> ReadOnlySignal<T> {
        ReadOnlySignal { inner: self }
    }

    pub fn id(&self) -> SignalId {
        self.id
    }
}

impl<T: 'static + Default> Default for Signal<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: 'static + fmt::Debug + Clone> fmt::Debug for Signal<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Signal")
            .field("id", &self.id)
            .field("value", &self.get_untracked())
            .finish()
    }
}

impl<T: 'static + std::ops::AddAssign<T> + Clone> std::ops::AddAssign<T> for Signal<T> {
    fn add_assign(&mut self, rhs: T) {
        self.update(|v| *v += rhs);
    }
}

impl<T: 'static + std::ops::SubAssign<T> + Clone> std::ops::SubAssign<T> for Signal<T> {
    fn sub_assign(&mut self, rhs: T) {
        self.update(|v| *v -= rhs);
    }
}

impl<T: 'static + std::ops::MulAssign<T> + Clone> std::ops::MulAssign<T> for Signal<T> {
    fn mul_assign(&mut self, rhs: T) {
        self.update(|v| *v *= rhs);
    }
}

impl<T: 'static + std::ops::DivAssign<T> + Clone> std::ops::DivAssign<T> for Signal<T> {
    fn div_assign(&mut self, rhs: T) {
        self.update(|v| *v /= rhs);
    }
}

pub struct ReadOnlySignal<T> {
    inner: Signal<T>,
}

impl<T> Copy for ReadOnlySignal<T> {}

impl<T> Clone for ReadOnlySignal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> PartialEq for ReadOnlySignal<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T> Eq for ReadOnlySignal<T> {}

impl<T> Hash for ReadOnlySignal<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl<T: 'static> ReadOnlySignal<T> {
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.inner.get()
    }

    pub fn get_untracked(&self) -> T
    where
        T: Clone,
    {
        self.inner.get_untracked()
    }

    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.inner.with(f)
    }

    pub fn with_untracked<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.inner.with_untracked(f)
    }

    pub fn subscribe(&self, callback: impl Fn() + 'static) {
        self.inner.subscribe(callback);
    }
}

impl<T: 'static + fmt::Debug + Clone> fmt::Debug for ReadOnlySignal<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReadOnlySignal")
            .field("value", &self.get_untracked())
            .finish()
    }
}
