// Copyright 2018 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fmt;
use future::Future;
use std::marker::{PhantomData, Unpin};
use std::mem::PinMut;
use task::{Context, Poll};
use spawn::Spawn;

/// A custom trait object for polling futures, roughly akin to
/// `Box<dyn Future<Output = T> + 'a>`.
///
/// This custom trait object was introduced for two reasons:
/// - Currently it is not possible to take `dyn Trait` by value and
///   `Box<dyn Trait>` is not available in no_std contexts.
/// - The `Future` trait is currently not object safe: The `Future::poll`
///   method makes uses the arbitrary self types feature and traits in which
///   this feature is used are currently not object safe due to current compiler
///   limitations. (See tracking issue for arbitray self types for more
///   information #44874)
pub struct LocalFutureObj<'a, T, S: Spawn + ?Sized> {
    ptr: *mut (),
    poll_fn: unsafe fn(*mut (), &mut Context<S>) -> Poll<T>,
    drop_fn: unsafe fn(*mut ()),
    _marker: PhantomData<&'a ()>,
}

impl<'a, T, S: Spawn + ?Sized> Unpin for LocalFutureObj<'a, T, S> {}

impl<'a, T, S: Spawn + ?Sized> LocalFutureObj<'a, T, S> {
    /// Create a `LocalFutureObj` from a custom trait object representation.
    #[inline]
    pub fn new<F: UnsafeFutureObj<'a, T, S> + 'a>(f: F) -> LocalFutureObj<'a, T, S> {
        LocalFutureObj {
            ptr: f.into_raw(),
            poll_fn: F::poll,
            drop_fn: F::drop,
            _marker: PhantomData,
        }
    }

    /// Converts the `LocalFutureObj` into a `FutureObj`
    /// To make this operation safe one has to ensure that the `UnsafeFutureObj`
    /// instance from which this `LocalFutureObj` was created actually
    /// implements `Send`.
    #[inline]
    pub unsafe fn into_future_obj(self) -> FutureObj<'a, T, S> {
        FutureObj(self)
    }
}

impl<'a, T, S: Spawn + ?Sized> fmt::Debug for LocalFutureObj<'a, T, S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("LocalFutureObj")
            .finish()
    }
}

impl<'a, T, S: Spawn + ?Sized> From<FutureObj<'a, T, S>> for LocalFutureObj<'a, T, S> {
    #[inline]
    fn from(f: FutureObj<'a, T, S>) -> LocalFutureObj<'a, T, S> {
        f.0
    }
}

impl<'a, T, S: Spawn + ?Sized> Future<S> for LocalFutureObj<'a, T, S> {
    type Output = T;

    #[inline]
    fn poll(self: PinMut<Self>, cx: &mut Context<S>) -> Poll<T> {
        unsafe {
            (self.poll_fn)(self.ptr, cx)
        }
    }
}

impl<'a, T, S: Spawn + ?Sized> Drop for LocalFutureObj<'a, T, S> {
    fn drop(&mut self) {
        unsafe {
            (self.drop_fn)(self.ptr)
        }
    }
}

/// A custom trait object for polling futures, roughly akin to
/// `Box<dyn Future<Output = T> + Send + 'a>`.
///
/// This custom trait object was introduced for two reasons:
/// - Currently it is not possible to take `dyn Trait` by value and
///   `Box<dyn Trait>` is not available in no_std contexts.
/// - The `Future` trait is currently not object safe: The `Future::poll`
///   method makes uses the arbitrary self types feature and traits in which
///   this feature is used are currently not object safe due to current compiler
///   limitations. (See tracking issue for arbitray self types for more
///   information #44874)
pub struct FutureObj<'a, T, S: Spawn + ?Sized>(LocalFutureObj<'a, T, S>);

impl<'a, T, S: Spawn + ?Sized> Unpin for FutureObj<'a, T, S> {}
unsafe impl<'a, T, S: Spawn> Send for FutureObj<'a, T, S> {}

impl<'a, T, S: Spawn + ?Sized> FutureObj<'a, T, S> {
    /// Create a `FutureObj` from a custom trait object representation.
    #[inline]
    pub fn new<F: UnsafeFutureObj<'a, T, S> + Send>(f: F) -> FutureObj<'a, T, S> {
        FutureObj(LocalFutureObj::new(f))
    }
}

impl<'a, T, S: Spawn + ?Sized> fmt::Debug for FutureObj<'a, T, S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("FutureObj")
            .finish()
    }
}

impl<'a, T, S: Spawn + ?Sized> Future<S> for FutureObj<'a, T, S> {
    type Output = T;

    #[inline]
    fn poll(self: PinMut<Self>, cx: &mut Context<S>) -> Poll<T> {
        let pinned_field = unsafe { PinMut::map_unchecked(self, |x| &mut x.0) };
        pinned_field.poll(cx)
    }
}

/// A custom implementation of a future trait object for `FutureObj`, providing
/// a hand-rolled vtable.
///
/// This custom representation is typically used only in `no_std` contexts,
/// where the default `Box`-based implementation is not available.
///
/// The implementor must guarantee that it is safe to call `poll` repeatedly (in
/// a non-concurrent fashion) with the result of `into_raw` until `drop` is
/// called.
pub unsafe trait UnsafeFutureObj<'a, T, S: Spawn + ?Sized>: 'a {
    /// Convert an owned instance into a (conceptually owned) void pointer.
    fn into_raw(self) -> *mut ();

    /// Poll the future represented by the given void pointer.
    ///
    /// # Safety
    ///
    /// The trait implementor must guarantee that it is safe to repeatedly call
    /// `poll` with the result of `into_raw` until `drop` is called; such calls
    /// are not, however, allowed to race with each other or with calls to
    /// `drop`.
    unsafe fn poll(ptr: *mut (), cx: &mut Context<S>) -> Poll<T>;

    /// Drops the future represented by the given void pointer.
    ///
    /// # Safety
    ///
    /// The trait implementor must guarantee that it is safe to call this
    /// function once per `into_raw` invocation; that call cannot race with
    /// other calls to `drop` or `poll`.
    unsafe fn drop(ptr: *mut ());
}

unsafe impl<'a, T, F, S: Spawn + ?Sized> UnsafeFutureObj<'a, T, S> for &'a mut F
    where F: Future<S, Output = T> + Unpin + 'a
{
    fn into_raw(self) -> *mut () {
        self as *mut F as *mut ()
    }

    unsafe fn poll(ptr: *mut (), cx: &mut Context<S>) -> Poll<T> {
        PinMut::new_unchecked(&mut *(ptr as *mut F)).poll(cx)
    }

    unsafe fn drop(_ptr: *mut ()) {}
}