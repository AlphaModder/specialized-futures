#![feature(futures_api, pin, arbitrary_self_types)]

mod future;
pub use self::future::{Future, FutureObj, LocalFutureObj, UnsafeFutureObj};

mod task;
pub use self::task::Context;

mod spawn;
pub use self::spawn::{Spawn, SpawnLocal};