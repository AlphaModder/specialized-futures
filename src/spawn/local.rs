use spawn::{Spawn, SpawnObjError};
use future::LocalFutureObj;

pub trait SpawnLocal: Spawn {
    fn spawn_obj_local(
        &mut self,
        future: LocalFutureObj<'static, (), dyn Spawn>
    ) -> Result<(), SpawnObjError<LocalFutureObj<'static, (), dyn Spawn>>>;
}


