use slotmap::{new_key_type, SlotMap};
use std::any::Any;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::rc::Rc;

new_key_type! {
    pub struct SignalId;
}

pub(crate) struct SignalValue {
    pub value: Box<dyn Any>,
    pub generation: u32,
}

pub(crate) type Subscriber = Rc<dyn Fn()>;

pub(crate) struct SignalStorage {
    values: SlotMap<SignalId, SignalValue>,
    subscribers: BTreeMap<SignalId, Vec<Subscriber>>,
    dependencies: BTreeMap<SignalId, HashSet<SignalId>>,
    current_observer: Option<SignalId>,
}

impl SignalStorage {
    pub fn new() -> Self {
        Self {
            values: SlotMap::with_key(),
            subscribers: BTreeMap::new(),
            dependencies: BTreeMap::new(),
            current_observer: None,
        }
    }

    pub fn insert<T: 'static>(&mut self, value: T) -> SignalId {
        let signal_value = SignalValue {
            value: Box::new(value),
            generation: 0,
        };
        self.values.insert(signal_value)
    }

    pub fn get<T: 'static>(&self, id: SignalId, generation: u32) -> Option<&T> {
        self.values.get(id).and_then(|signal_value| {
            if signal_value.generation == generation {
                signal_value.value.downcast_ref()
            } else {
                None
            }
        })
    }

    pub fn get_mut<T: 'static>(&mut self, id: SignalId, generation: u32) -> Option<&mut T> {
        self.values.get_mut(id).and_then(|signal_value| {
            if signal_value.generation == generation {
                signal_value.value.downcast_mut()
            } else {
                None
            }
        })
    }

    pub fn set<T: 'static>(
        &mut self,
        id: SignalId,
        generation: u32,
        value: T,
    ) -> Option<Vec<Subscriber>> {
        if let Some(signal_value) = self.values.get_mut(id)
            && signal_value.generation == generation
        {
            signal_value.value = Box::new(value);
            let callbacks: Vec<Subscriber> = self
                .subscribers
                .get(&id)
                .map(|subs| subs.to_vec())
                .unwrap_or_default();
            return Some(callbacks);
        }
        None
    }

    pub fn update<T: 'static, R>(
        &mut self,
        id: SignalId,
        generation: u32,
        f: impl FnOnce(&mut T) -> R,
    ) -> Option<(R, Vec<Subscriber>)> {
        if let Some(value) = self.get_mut::<T>(id, generation) {
            let result = f(value);
            let callbacks = self
                .subscribers
                .get(&id)
                .map(|subs| subs.to_vec())
                .unwrap_or_default();
            Some((result, callbacks))
        } else {
            None
        }
    }

    pub fn subscribe(&mut self, id: SignalId, callback: impl Fn() + 'static) {
        self.subscribers
            .entry(id)
            .or_default()
            .push(Rc::new(callback));
    }

    pub fn track_read(&mut self, id: SignalId) {
        if let Some(observer_id) = self.current_observer {
            let deps = self.dependencies.entry(observer_id).or_default();
            if deps.insert(id) {
                let observer_ptr = observer_id;
                self.subscribe(id, move || notify_subscribers(observer_ptr));
            }
        }
    }

    pub fn set_observer(&mut self, observer: Option<SignalId>) -> Option<SignalId> {
        std::mem::replace(&mut self.current_observer, observer)
    }
}

thread_local! {
    static STORAGE: RefCell<SignalStorage> = RefCell::new(SignalStorage::new());
}

pub(crate) fn with_signal_storage<R>(f: impl FnOnce(&mut SignalStorage) -> R) -> R {
    STORAGE.with(|storage| f(&mut storage.borrow_mut()))
}

pub(crate) fn notify_subscribers(id: SignalId) {
    let callbacks: Vec<Subscriber> = with_signal_storage(|storage| {
        storage
            .subscribers
            .get(&id)
            .map(|subs| subs.to_vec())
            .unwrap_or_default()
    });

    for callback in callbacks {
        callback();
    }
}
