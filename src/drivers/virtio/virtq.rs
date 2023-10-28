use alloc::boxed::Box;

use core::future::poll_fn;
use core::intrinsics::unaligned_volatile_store;
use core::sync::atomic::{fence, Ordering};
use core::task::{Poll, Context};

use futures_util::future::BoxFuture;
use futures_util::task::AtomicWaker;

use super::pci::ComCfgRaw;

pub const VIRTQ_DESC_F_NEXT: u16 = 1 << 0;
pub const VIRTQ_DESC_F_WRITE: u16 = 1 << 1;
pub const VIRTQ_DESC_F_AVAIL: u16 = 1 << 7;
pub const VIRTQ_DESC_F_USED: u16 = 1 << 15;

#[repr(C, align(16))]
#[derive(Default, Debug, Copy, Clone)]
pub struct Descriptor {
	pub address: u64,
	pub len: u32,
	pub flags: u16,
	pub next: u16
}

type DescriptorRing<const COUNT: usize> = [Descriptor; COUNT];

#[repr(C, align(2))]
struct AvailRing<const COUNT: usize> {
	flags: u16,
	index: u16,
	ring: [u16; COUNT],
	event: u16,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
struct UsedElem {
	id: u32,
	len: u32,
}

#[repr(C, align(2))]
struct UsedRing<const COUNT: usize> {
	flags: u16,
	index: u16,
	ring: [UsedElem; COUNT],
	event: u16,
}

struct StaticQueue<const COUNT: usize> {
    avail_wrap_count: bool,
    state: [bool; COUNT],
    descr_head: usize,
    descr_current: usize,
    descr_next: usize,
    avail_next: usize,
    notify_addr: *mut u16,
    wakers: [AtomicWaker; COUNT],
    descriptors: DescriptorRing<COUNT>,
    avail: AvailRing<COUNT>,
    used_last: usize,
    used: UsedRing<COUNT>,
}

type Queue128 = StaticQueue<128>;
type Queue256 = StaticQueue<256>;
type Queue512 = StaticQueue<512>;

// notify_addr can be shared between threads
unsafe impl<const COUNT: usize> Send for StaticQueue<COUNT> {}
unsafe impl<const COUNT: usize> Sync for StaticQueue<COUNT> {}

pub struct Virtq {
    queues: Box<dyn Queue>
}

pub struct VirtqHandler<'a> {
    index: u16,
    notify_addr: *mut u16,
    raw: &'a mut ComCfgRaw
}

trait Queue: Send + Sync {
    fn get_addresses(&self) -> (u64, u64, u64);
    fn request<'a>(&'a mut self, desc: &'a [Descriptor]) -> BoxFuture<'a, u16>;
    fn process(&mut self);
}

impl<'a> VirtqHandler<'a> {
    pub fn new(raw: &'a mut ComCfgRaw, index: u16, notify_addr: *mut u16) -> Self {
        Self {
            index,
            notify_addr,
            raw
        }
    }

    pub fn set_vq_size(&mut self, size: u16) -> u16 {
        self.raw.queue_select = self.index;
        if self.raw.queue_size > size {
            self.raw.queue_size = size;
        }

        self.raw.queue_size
    }

	pub fn set_desc_ring_addr(&mut self, addr: u64) {
		self.raw.queue_select = self.index;
		self.raw.queue_desc = addr as u64;
	}

	pub fn set_avail_ring_addr(&mut self, addr: u64) {
		self.raw.queue_select = self.index;
		self.raw.queue_driver = addr as u64;
	}

	pub fn set_used_ring_addr(&mut self, addr: u64) {
		self.raw.queue_select = self.index;
		self.raw.queue_device = addr as u64;
	}

    pub fn set_msix_vector(&mut self, vector: u16) {
        self.raw.queue_select = self.index;
		self.raw.queue_msix_vector = vector;
    }

    pub fn enable_queue(&mut self) {
		self.raw.queue_select = self.index;
		self.raw.queue_enable = 1;
	}

    pub fn get_notify_addr(&self) -> *mut u16 {
        self.notify_addr
    }
}

impl Descriptor {
    pub fn new<T>(data: &mut T, flags: u16) -> Self {
        Self {
            address: data as *mut T as u64,
            len: core::mem::size_of::<T>() as u32,
            flags,
            next: 0
        }
    }

    pub fn new_raw<T>(data: *mut T, len: usize, flags: u16) -> Self {
        Self {
            address: data as u64,
            len: len as u32,
            flags,
            next: 0
        }
    }
}

impl<const COUNT: usize> StaticQueue<COUNT> {
    pub fn new(notify_addr: *mut u16) -> Self {
        let mut queues = Self {
            avail_wrap_count: true,
            state: [false; COUNT],
            descr_head: 0,
            descr_current: 0,
            descr_next: 0,
            avail_next: 0,
            notify_addr,
            wakers: core::array::from_fn(|_| AtomicWaker::new()),
            descriptors: [Descriptor::default(); COUNT],
            avail: AvailRing {
                flags: 0,
                index: 0,
                ring: [0; COUNT],
                event: 0,
            },
            used_last: 0,
            used: UsedRing {
                flags: 0,
                index: 0,
                ring: [UsedElem::default(); COUNT],
                event: 0,
            }
        };
        for i in 0..COUNT {
            queues.descriptors[i].next = (i + 1) as u16;
        }
        queues
    }

    pub fn insert_descriptor(&mut self, desc: &Descriptor) {
        // Track next available free descriptor
        let next_free = self.descriptors[self.descr_next].next as usize;

        self.descriptors[self.descr_next] = *desc;
        if self.avail_wrap_count {
            self.descriptors[self.descr_next].flags |= VIRTQ_DESC_F_AVAIL;
            self.descriptors[self.descr_next].flags &= !VIRTQ_DESC_F_USED;
        } else {
            self.descriptors[self.descr_next].flags &= !VIRTQ_DESC_F_AVAIL;
            self.descriptors[self.descr_next].flags |= VIRTQ_DESC_F_USED;
        }

        if self.descr_current != self.descr_next {
            // Update previous descriptor to chain descriptors
            self.descriptors[self.descr_current].flags |= VIRTQ_DESC_F_NEXT;
            self.descriptors[self.descr_current].next = self.descr_next as u16;
            self.descr_current = self.descr_next;
        }

        self.descr_next = next_free;
    }

    pub fn process_used_queue(&mut self, ignore: usize) {
        while self.used_last != self.used.index as usize {
            // Notify other sleeping pollers
            let idx = self.used.ring[self.used_last].id as usize;
            self.state[idx] = true;

            if ignore != idx {
                self.wakers[idx].wake();
            }

            // Update counter
            self.used_last = (self.used_last + 1) % self.descriptors.len();
            if self.used_last == 0 {
                self.avail_wrap_count = !self.avail_wrap_count;
            }
        }
    }

    pub fn poll_request(&mut self, ctx: &mut Context, head: usize) -> Poll<()> {
        if self.state[head] {
            return Poll::Ready(())
        }

        self.wakers[head].register(&ctx.waker());
        if self.state[head] {
            Poll::Ready(())
        } else {
            self.wakers[head].wake();
            Poll::Pending
        }
    }

    pub fn notify(&self, val: u16) {
        fence(Ordering::Acquire);

        unsafe { unaligned_volatile_store(self.notify_addr, val); }

        fence(Ordering::Release);
    }
}

impl<const COUNT: usize> Queue for StaticQueue<COUNT> {
    fn get_addresses(&self) -> (u64, u64, u64) {
        (self.descriptors.as_ptr() as u64, &self.avail as *const AvailRing<COUNT> as u64, &self.used as *const UsedRing<COUNT> as u64)
    }

    fn request<'a>(&'a mut self, desc: &'a [Descriptor]) -> BoxFuture<'a, u16> {
        Box::pin(async {
            desc.iter().for_each(|d| self.insert_descriptor(d));
            self.avail.ring[self.avail_next] = self.descr_head as u16;
            self.avail_next += 1;

            // Write barrier so that device sees changes to descriptor table and available ring
            fence(Ordering::SeqCst);

            self.avail.index += 1;

            // Write barrier so that device can see change to available index after this method returns.
            fence(Ordering::SeqCst);

            // Start new chain
            let current_chain = self.descr_head;
            self.descr_head = self.descr_next;
            self.descr_current = self.descr_next;

            // Notify device about new descriptor chain
            self.notify(current_chain as u16);

            poll_fn(|ctx| self.poll_request(ctx, current_chain)).await;
            current_chain as u16
        })
    }

    fn process(&mut self) {
        self.process_used_queue(usize::MAX);
    }
}

impl Virtq {
    pub fn new(handler: &mut VirtqHandler<'_>) -> Self {
        let size = handler.set_vq_size(256);

        let queues: Box<dyn Queue> = if size == 128 {
            Box::new(Queue128::new(handler.get_notify_addr()))
        } else if size == 256 {
            Box::new(Queue256::new(handler.get_notify_addr()))
        } else if size == 512 {
            Box::new(Queue512::new(handler.get_notify_addr()))
        } else {
            panic!("Invalid queue size!");
        };

        let (desc_ring_addr, avail_ring_addr, used_ring_addr) = queues.get_addresses();

        handler.set_desc_ring_addr(desc_ring_addr);
        handler.set_avail_ring_addr(avail_ring_addr);
        handler.set_used_ring_addr(used_ring_addr);
        handler.enable_queue();

        Self {
            queues
        }
    }

    pub async fn request(&mut self, descs: &[Descriptor]) -> Result<(), &'static str> {
        self.queues.request(descs).await;

        Ok(())
    }

    pub fn process(&mut self) {
        self.queues.process();
    }
}
