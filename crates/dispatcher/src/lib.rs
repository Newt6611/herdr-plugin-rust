//! Generic typed async event dispatcher.

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    future::Future,
    marker::PhantomData,
    pin::Pin,
};

type BoxFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

/// TypeId-backed async event dispatcher.
pub struct EventDispatcher<C> {
    handlers: HashMap<TypeId, Vec<HandlerEntry<C>>>,
    context: PhantomData<fn(C)>,
}

impl<C> Default for EventDispatcher<C> {
    fn default() -> Self {
        Self {
            handlers: HashMap::new(),
            context: PhantomData,
        }
    }
}

impl<C> EventDispatcher<C>
where
    C: Clone + Send + Sync + 'static,
{
    /// Creates an empty dispatcher.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an async handler for a concrete event type.
    pub fn on<E>(&mut self, handler: impl Handler<C, E>) -> &mut Self
    where
        E: Clone + Send + Sync + 'static,
    {
        self.handlers
            .entry(TypeId::of::<E>())
            .or_default()
            .push(HandlerEntry::new(TypedHandler::<C, E, _> {
                handler,
                context: PhantomData,
                event: PhantomData,
            }));
        self
    }

    /// Dispatches an event to handlers registered for its concrete type.
    pub async fn dispatch<E>(&self, context: C, event: E)
    where
        E: Clone + Send + Sync + 'static,
    {
        let Some(handlers) = self.handlers.get(&TypeId::of::<E>()) else {
            return;
        };

        for entry in handlers {
            entry.handler.call(context.clone(), &event).await;
        }
    }
}

struct HandlerEntry<C> {
    handler: Box<dyn ErasedHandler<C>>,
}

impl<C> HandlerEntry<C> {
    fn new<H>(handler: H) -> Self
    where
        H: ErasedHandler<C>,
    {
        Self {
            handler: Box::new(handler),
        }
    }
}

/// A typed async event handler.
///
/// Function items and closures matching `Fn(C, E) -> Future<Output = ()>`
/// implement this trait automatically.
pub trait Handler<C, E>: Send + Sync + 'static {
    #[doc(hidden)]
    fn call(&self, context: C, event: E) -> BoxFuture;
}

impl<C, E, F, Fut> Handler<C, E> for F
where
    C: Send + 'static,
    E: Send + 'static,
    F: Fn(C, E) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    fn call(&self, context: C, event: E) -> BoxFuture {
        Box::pin((self)(context, event))
    }
}

trait ErasedHandler<C>: Send + Sync + 'static {
    fn call(&self, context: C, event: &(dyn Any + Send + Sync)) -> BoxFuture;
}

struct TypedHandler<C, E, F> {
    handler: F,
    context: PhantomData<fn(C)>,
    event: PhantomData<fn(E)>,
}

impl<C, E, H> ErasedHandler<C> for TypedHandler<C, E, H>
where
    C: Send + 'static,
    E: Clone + Send + Sync + 'static,
    H: Handler<C, E>,
{
    fn call(&self, context: C, event: &(dyn Any + Send + Sync)) -> BoxFuture {
        let event = event
            .downcast_ref::<E>()
            .expect("event type did not match registered handler")
            .clone();

        self.handler.call(context, event)
    }
}
