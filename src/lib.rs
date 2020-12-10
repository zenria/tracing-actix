//! Actix ActorFuture instrumentation for use with [`tracing`]
//!
//! # Example
//!
//! ```rust
//! use actix::prelude::*;
//! use actix::fut::{ready, ActorFuture};
//! use tracing::{span, event, Level};
//! use tracing_actix::ActorInstrument;
//! #
//! # // The response type returned by the actor future
//! # type OriginalActorResponse = ();
//! # // The error type returned by the actor future
//! # type MessageError = ();
//! # // This is the needed result for the DeferredWork message
//! # // It's a result that combine both Response and Error from the future response.
//! # type DeferredWorkResult = Result<OriginalActorResponse, MessageError>;
//! #
//! # struct ActorState {}
//! #
//! # impl ActorState {
//! #    fn update_from(&mut self, _result: ()) {}
//! # }
//! #
//! # struct OtherActor {}
//! #
//! # impl Actor for OtherActor {
//! #    type Context = Context<Self>;
//! # }
//! #
//! # impl Handler<OtherMessage> for OtherActor {
//! #    type Result = ();
//! #
//! #    fn handle(&mut self, _msg: OtherMessage, _ctx: &mut Context<Self>) -> Self::Result {
//! #    }
//! # }
//! #
//! # struct OriginalActor{
//! #     other_actor: Addr<OtherActor>,
//! #     inner_state: ActorState
//! # }
//! #
//! # impl Actor for OriginalActor{
//! #     type Context = Context<Self>;
//! # }
//! #
//! # #[derive(Message)]
//! # #[rtype(result = "Result<(), MessageError>")]
//! # struct DeferredWork{}
//! #
//! # #[derive(Message)]
//! # #[rtype(result = "()")]
//! # struct OtherMessage{}
//!
//! /// This example is modified from the actix::fut module and intended to show how
//! /// the `ActorInstrument` trait may be used to integrate tracing `Span`s within
//! /// asynchronous message handlers.
//! impl Handler<DeferredWork> for OriginalActor {
//!     type Result = ResponseActFuture<Self, Result<OriginalActorResponse, MessageError>>;
//!
//!     fn handle(
//!         &mut self,
//!         _msg: DeferredWork,
//!         _ctx: &mut Context<Self>,
//!     ) -> Self::Result {
//!         // this creates a `Future` representing the `.send` and subsequent `Result` from
//!         // `other_actor`
//!         let span = span!(Level::INFO, "deferred work context");
//!         // Addr<A>::send returns `actix::prelude::Request`, which implements Unpin, so we can wrap
//!         // into_actor within an ActorInstrument.
//!         Box::pin(
//!             self.other_actor
//!                 .send(OtherMessage {})
//!                 .into_actor(self)
//!                 .actor_instrument(span)
//!                 .map(|result, actor, _ctx| {
//!                     // Actor's state updated here
//!                     match result {
//!                         Ok(v) => {
//!                             event!(Level::INFO, "I'm within deferred work context");
//!                             actor.inner_state.update_from(v);
//!                             Ok(())
//!                         }
//!                         // Failed to send message to other_actor
//!                         Err(e) => {
//!                             event!(Level::ERROR, "Error from deferred work: {:?}", e);
//!                             Err(())
//!                         }
//!                     }
//!                 }),
//!         )
//!     }
//! }
//! #
//! # #[derive(Message)]
//! # #[rtype(result = "Pong")]
//! # struct Ping;
//! #
//! # struct Pong;
//!
//! /// In this example, there isn't an `actix::prelude::Request` in our `ActorFuture`.
//! /// Since `ActorInstrument` needs to wrap `ActorFuture + Unpin`, we can't use
//! /// `async {}.into_actor(self)` because `async {}` doesn't implement `Unpin`.
//! impl Handler<Ping> for OriginalActor {
//!     type Result = ResponseActFuture<Self, Pong>;
//!
//!     fn handle(
//!         &mut self,
//!         _msg: Ping,
//!         _ctx: &mut Context<Self>,
//!     ) -> Self::Result {
//!         // `async {}` doesn't implement Unpin, so it can't be used.
//!         // `actix::fut::Ready` ActorFutures work fine though.
//!         let span = span!(Level::INFO, "ping");
//!         Box::pin(
//!             ready::<Pong, Self>(Pong {})
//!                 .actor_instrument(span)
//!                 .map(|pong, _this, _ctx| {
//!                     // the pong event occurs in the ping span, even though this is async.
//!                     event!(Level::INFO, "pong");
//!                     pong
//!                 }),
//!         )
//!     }
//! }
//! ```
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
    fn actor_instrument(self, span: Span) -> ActorInstrumented<Self> {
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
