use core::any::{Any, TypeId};

use ahash::RandomState;

use alloc::sync::Arc;

use hashbrown::HashMap;

use spin::{Once, Mutex, RwLock};

use crate::arch::Arch;
use crate::drivers::i8042::PcKeyboard;
use crate::drivers::video::console::Console;
use crate::drivers::video::fb::FrameBuffer;
use crate::scheduler::Scheduler;

pub static RUNTIME: Once<Runtime> = Once::new();

pub trait Resource: Any + Send + Sync {}

pub struct Runtime {
    pub system: Arch,
    pub scheduler: Scheduler,
    pub console: Mutex<Console>,
    pub keyboard: PcKeyboard,
    resources: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>, RandomState>>
}

impl Runtime {
    pub fn init(system: Arch, fb: FrameBuffer, kbd: PcKeyboard) -> &'static Self {
        RUNTIME.call_once(|| {
            Runtime {
                system,
                scheduler: Scheduler::new(),
                console: Mutex::new(Console::new(fb)),
                keyboard: kbd,
                resources: RwLock::new(HashMap::with_hasher(RandomState::new()))
            }
        })
    }

    pub fn register<T: Resource>(&self, resource: Arc<T>) {
        self.resources.write().insert(TypeId::of::<T>(), resource);
    }

    pub fn get<T: Resource>(&self) -> Option<Arc<T>> {
        self.resources.read().get(&TypeId::of::<T>()).map(|r| r.clone().downcast::<T>().unwrap())
    }
}

pub fn runtime() -> &'static Runtime {
    RUNTIME.wait()
}
