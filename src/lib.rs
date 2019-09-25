//! Actix ActorFuture instrumentation for use with [`tracing`]
//!

use actix::{Actor, ActorFuture};
use futures::Async;
use tracing::Span;

pub trait ActorInstrument: Sized {
    /// Instruments this type with the provided `Span`, returning an
    /// `ActorInstrumented` wrapper.
    ///
    /// When the wrapped actor future is polled, the attached `Span`
    /// will be entered for the duration of the poll.
    fn actor_instrument(self, span: Span) -> ActorInstrumented<Self> {
        ActorInstrumented { inner: self, span }
    }
}

impl<T: ActorFuture> ActorInstrument for T {}

/// An actor future that has been instrumented with a `tracing` span.
#[derive(Debug, Clone)]
pub struct ActorInstrumented<T> {
    inner: T,
    span: Span,
}
impl<T: ActorFuture> ActorFuture for ActorInstrumented<T> {
    type Item = T::Item;
    type Error = T::Error;
    type Actor = T::Actor;

    fn poll(
        &mut self,
        srv: &mut Self::Actor,
        ctx: &mut <Self::Actor as Actor>::Context,
    ) -> Result<Async<Self::Item>, Self::Error> {
        let _enter = self.span.enter();
        self.inner.poll(srv, ctx)
    }
}

impl<T> ActorInstrumented<T> {
    /// Borrows the `Span` that this type is instrumented by.
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Mutably borrows the `Span` that this type is instrumented by.
    pub fn span_mut(&mut self) -> &mut Span {
        &mut self.span
    }

    /// Consumes the `Instrumented`, returning the wrapped type.
    ///
    /// Note that this drops the span.
    pub fn into_inner(self) -> T {
        self.inner
    }
}
