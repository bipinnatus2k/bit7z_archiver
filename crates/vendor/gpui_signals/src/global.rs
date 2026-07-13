use crate::context::auto_notify;
use crate::Signal;
use gpui::{App, Context, Global};

struct GlobalSignalContainer<T: 'static> {
    signal: Signal<T>,
}

impl<T: 'static> Global for GlobalSignalContainer<T> {}

pub trait GlobalSignalContext {
    fn init_global<T: 'static>(&mut self, initial_value: T) -> Signal<T>;
    fn global_signal<T: 'static>(&self) -> Signal<T>;
    fn use_global<T: 'static>(&mut self) -> Signal<T>;
}

impl GlobalSignalContext for App {
    fn init_global<T: 'static>(&mut self, initial_value: T) -> Signal<T> {
        let signal = Signal::new(initial_value);
        self.set_global(GlobalSignalContainer { signal });
        signal
    }

    fn global_signal<T: 'static>(&self) -> Signal<T> {
        self.global::<GlobalSignalContainer<T>>().signal
    }

    fn use_global<T: 'static>(&mut self) -> Signal<T> {
        self.global_signal::<T>()
    }
}

impl<V: 'static> GlobalSignalContext for Context<'_, V> {
    fn init_global<T: 'static>(&mut self, initial_value: T) -> Signal<T> {
        let signal = Signal::new(initial_value);
        self.set_global(GlobalSignalContainer { signal });
        signal
    }

    fn global_signal<T: 'static>(&self) -> Signal<T> {
        self.global::<GlobalSignalContainer<T>>().signal
    }

    fn use_global<T: 'static>(&mut self) -> Signal<T> {
        let signal = self.global_signal::<T>();
        if crate::context::subscribe_once(self, &signal) {
            let sub = auto_notify(&signal, self);
            crate::context::track_subscription(self, sub);
        }
        signal
    }
}
