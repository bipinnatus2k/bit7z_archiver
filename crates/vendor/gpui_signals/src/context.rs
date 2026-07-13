use crate::storage::SignalId;
use crate::{Memo, Signal};
use futures::channel::mpsc;
use futures::StreamExt;
use gpui::{EntityId, Subscription, WeakEntity};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

pub trait SignalContext {
    fn create_signal<T: 'static>(&mut self, initial: T) -> Signal<T>;
    fn create_memo<T: 'static + Clone>(&mut self, compute: impl Fn() -> T + 'static) -> Memo<T>;
    fn create_effect(&mut self, effect: impl Fn() + 'static);
}

thread_local! {
    static ENTITY_SUBSCRIPTIONS: RefCell<HashMap<EntityId, Vec<Subscription>>> = RefCell::new(HashMap::new());
    static ENTITY_CLEANUP_REGISTERED: RefCell<HashSet<EntityId>> = RefCell::new(HashSet::new());
    static ENTITY_SIGNAL_SUBSCRIPTIONS: RefCell<HashMap<EntityId, HashSet<SignalId>>> = RefCell::new(HashMap::new());
}

impl<T: 'static> SignalContext for gpui::Context<'_, T> {
    fn create_signal<U: 'static>(&mut self, initial: U) -> Signal<U> {
        let signal = Signal::new(initial);
        let subscription = auto_notify(&signal, self);
        track_subscription(self, subscription);
        signal
    }

    fn create_memo<U: 'static + Clone>(&mut self, compute: impl Fn() -> U + 'static) -> Memo<U> {
        let memo = Memo::new(compute);
        let subscription = auto_notify(&memo.signal(), self);
        track_subscription(self, subscription);
        memo
    }

    fn create_effect(&mut self, effect: impl Fn() + 'static) {
        let active = Rc::new(Cell::new(true));
        let active_flag = active.clone();
        let _effect = Memo::new(move || {
            if active_flag.get() {
                effect();
            }
        });
        let cleanup_sub = self.on_release(move |_, _| {
            active.set(false);
        });
        track_subscription(self, cleanup_sub);
    }
}

pub(crate) fn auto_notify<T, V>(signal: &Signal<T>, cx: &mut gpui::Context<V>) -> Subscription
where
    T: 'static,
    V: 'static,
{
    let (tx, mut rx) = mpsc::unbounded::<()>();

    signal.subscribe({
        let tx = tx.clone();
        move || {
            let _ = tx.unbounded_send(());
        }
    });

    let task = cx.spawn(
        async move |entity: WeakEntity<V>, cx: &mut gpui::AsyncApp| {
            while let Some(()) = rx.next().await {
                if let Some(entity) = entity.upgrade() {
                    entity.update(cx, |_, cx| {
                        cx.notify();
                    });
                } else {
                    break;
                }
            }
        },
    );

    Subscription::new(move || {
        task.detach();
    })
}

pub(crate) fn track_subscription<V: 'static>(cx: &mut gpui::Context<V>, subscription: Subscription) {
    let entity_id = cx.entity_id();
    ENTITY_SUBSCRIPTIONS.with(|subs| {
        subs.borrow_mut()
            .entry(entity_id)
            .or_insert_with(Vec::new)
            .push(subscription);
    });

    let needs_cleanup = ENTITY_CLEANUP_REGISTERED.with(|registered| {
        let mut registered = registered.borrow_mut();
        if registered.contains(&entity_id) {
            false
        } else {
            registered.insert(entity_id);
            true
        }
    });

    if needs_cleanup {
        let cleanup_sub = cx.on_release(move |_, _| {
            ENTITY_SUBSCRIPTIONS.with(|subs| {
                subs.borrow_mut().remove(&entity_id);
            });
            ENTITY_CLEANUP_REGISTERED.with(|registered| {
                registered.borrow_mut().remove(&entity_id);
            });
            ENTITY_SIGNAL_SUBSCRIPTIONS.with(|subs| {
                subs.borrow_mut().remove(&entity_id);
            });
        });
        ENTITY_SUBSCRIPTIONS.with(|subs| {
            subs.borrow_mut()
                .entry(entity_id)
                .or_insert_with(Vec::new)
                .push(cleanup_sub);
        });
    }
}

pub(crate) fn subscribe_once<V: 'static, T: 'static>(
    cx: &mut gpui::Context<V>,
    signal: &Signal<T>,
) -> bool {
    let entity_id = cx.entity_id();
    ENTITY_SIGNAL_SUBSCRIPTIONS.with(|subs| {
        let mut subs = subs.borrow_mut();
        let entry = subs.entry(entity_id).or_insert_with(HashSet::new);
        entry.insert(signal.id())
    })
}
