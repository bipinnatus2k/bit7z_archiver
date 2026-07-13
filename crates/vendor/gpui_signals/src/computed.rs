use crate::signal::Signal;
use crate::storage::with_signal_storage;
use gpui::{IntoElement, SharedString};
use std::hash::{Hash, Hasher};
use std::{cell::Cell, fmt, marker::PhantomData, rc::Rc};

pub struct Memo<T> {
    signal: Signal<T>,
    _phantom: PhantomData<fn() -> T>,
}

impl<T> Copy for Memo<T> {}

impl<T> Clone for Memo<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> PartialEq for Memo<T> {
    fn eq(&self, other: &Self) -> bool {
        self.signal == other.signal
    }
}

impl<T> Eq for Memo<T> {}

impl<T> Hash for Memo<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.signal.hash(state);
    }
}

impl<T: fmt::Display + Clone + 'static> IntoElement for Memo<T> {
    type Element = SharedString;

    fn into_element(self) -> Self::Element {
        self.get().to_string().into()
    }
}

impl<T: 'static + Clone> Memo<T> {
    pub(crate) fn new(compute: impl Fn() -> T + 'static) -> Self {
        let compute = Rc::new(compute);
        let recomputing = Rc::new(Cell::new(false));
        let signal = Signal::new(compute());
        let recompute_signal = signal;

        let recompute: Rc<dyn Fn()> = {
            let compute = compute.clone();
            let signal = recompute_signal;
            let recomputing = recomputing.clone();
            Rc::new(move || {
                if recomputing.replace(true) {
                    return;
                }
                let previous =
                    with_signal_storage(|storage| storage.set_observer(Some(signal.id())));
                let value = compute();
                with_signal_storage(|storage| storage.set_observer(previous));
                signal.set(value);
                recomputing.set(false);
            })
        };

        recompute();

        signal.subscribe({
            let recompute = recompute.clone();
            move || recompute()
        });

        Self {
            signal,
            _phantom: PhantomData,
        }
    }

    pub fn signal(&self) -> Signal<T> {
        self.signal
    }

    pub fn get(&self) -> T {
        self.signal.get()
    }

    pub fn get_untracked(&self) -> T {
        self.signal.get_untracked()
    }

    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.signal.with(f)
    }

    pub fn with_untracked<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.signal.with_untracked(f)
    }

    pub fn subscribe(&self, callback: impl Fn() + 'static) {
        self.signal.subscribe(callback);
    }
}

impl<T: 'static + Clone + fmt::Debug> fmt::Debug for Memo<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Memo")
            .field("value", &self.get_untracked())
            .finish()
    }
}
