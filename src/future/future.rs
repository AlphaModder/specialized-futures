use std::mem::PinMut;
use std::marker::Unpin;
use task::{Context, Poll};
use spawn::Spawn;

pub trait Future<S: Spawn + ?Sized = dyn Spawn> {
    type Output;

    fn poll(self: PinMut<Self>, cx: &mut Context<S>) -> Poll<Self::Output>;
}

impl<'a, S: Spawn + ?Sized, F: ?Sized + Future<S> + Unpin> Future<S> for &'a mut F {
    type Output = F::Output;

    fn poll(mut self: PinMut<Self>, cx: &mut Context<S>) -> Poll<Self::Output> {
        F::poll(PinMut::new(&mut **self), cx)
    }
}

impl<'a, S: Spawn + ?Sized, F: ?Sized + Future<S>> Future<S> for PinMut<'a, F> {
    type Output = F::Output;

    fn poll(mut self: PinMut<Self>, cx: &mut Context<S>) -> Poll<Self::Output> {
        F::poll((*self).reborrow(), cx)
    }
}