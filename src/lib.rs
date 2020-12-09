//! Actix ActorFuture instrumentation for use with [`tracing`]
//!
use actix::{Actor, ActorFuture};
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tracing::Span;

/// Extension trait allowing actor futures to be instrumented with
/// a `tracing` `Span`.
pub trait ActorInstrument: ActorFuture + Unpin + Sized {
    /// Instruments this type with the provided `Span`, returning an
    /// `ActorInstrumented` wrapper.
    ///
    /// When the wrapped actor future is polled, the attached `Span`
    /// will be entered for the duration of the poll.
    fn actor_instrument(
        self,
        span: Span,
    ) -> ActorInstrumented<Self> {
        ActorInstrumented { inner: self, span }
    }
}

impl<T: ActorFuture + Unpin + Sized> ActorInstrument for T {}

/// An actor future that has been instrumented with a `tracing` span.
#[derive(Debug, Clone)]
pub struct ActorInstrumented<T>
where
    T: ActorFuture,
{
    inner: T,
    span: Span,
}

impl<T: ActorFuture + Unpin> ActorFuture for ActorInstrumented<T> {
    type Output = <T as ActorFuture>::Output;
    type Actor = <T as ActorFuture>::Actor;

    fn poll(
        mut self: Pin<&mut Self>,
        srv: &mut Self::Actor,
        ctx: &mut <Self::Actor as Actor>::Context,
        task: &mut Context<'_>,
    ) -> Poll<Self::Output> {
        let span = self.span.clone();
        let _enter = span.enter();
        ActorFuture::poll(Pin::new(self.actor_mut()), srv, ctx, task)
    }
}

impl<T: ActorFuture + Unpin> ActorInstrumented<T> {
    /// Borrows the `Span` that this type is instrumented by.
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Mutably borrows the `Span` that this type is instrumented by.
    pub fn span_mut(&mut self) -> &mut Span {
        &mut self.span
    }

    pub fn actor_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}
