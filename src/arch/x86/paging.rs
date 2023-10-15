use alloc::alloc;

use core::alloc::Layout;

use x86_64::structures::paging::{FrameAllocator, Mapper, OffsetPageTable, Page, PageSize, PageTable, PageTableFlags, PhysFrame, Size4KiB};
use x86_64::{PhysAddr, VirtAddr};

#[derive(Default)]
struct OffsetAllocatorFrameAllocation {
    offset: usize,
}

unsafe impl<S: PageSize> FrameAllocator<S> for OffsetAllocatorFrameAllocation {
    fn allocate_frame(&mut self) -> Option<PhysFrame<S>> {
        let layout = Layout::from_size_align(S::SIZE as usize, S::SIZE as usize).unwrap();
        Some(unsafe {
            let mut ptr = alloc::alloc_zeroed(layout);
            ptr = ptr.wrapping_byte_sub(self.offset);
            PhysFrame::from_start_address(PhysAddr::new(ptr as u64)).unwrap()
        })
    }
}

pub struct PageMapper {
    alloc: OffsetAllocatorFrameAllocation,
    page_table: PhysFrame<Size4KiB>,
    mapper: OffsetPageTable<'static>,
}

impl PageMapper {
    pub fn new(offset: u64) -> Self {
        let mut alloc = OffsetAllocatorFrameAllocation {
            offset: offset as usize,
        };
        let page_frame: PhysFrame<Size4KiB> = alloc.allocate_frame().unwrap();
        let ptr = VirtAddr::new(page_frame.start_address().as_u64() + offset).as_mut_ptr();
        unsafe { *ptr = PageTable::new() };
        let page_table = unsafe { &mut *ptr };

        let mapper = unsafe { OffsetPageTable::new(page_table, VirtAddr::new(offset)) };

        Self {
            alloc,
            page_table: page_frame,
            mapper,
        }
    }

    pub unsafe fn map(&mut self, from: usize, to: usize, length: usize) {
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

        for i in 0..((length as u64) / Size4KiB::SIZE) {
            let start_frame = PhysFrame::<Size4KiB>::from_start_address(PhysAddr::new(from as u64 + i * Size4KiB::SIZE as u64)).unwrap();
            let map_frame = Page::from_start_address(VirtAddr::new(to as u64 + i * Size4KiB::SIZE)).unwrap();
            self.mapper.map_to(map_frame, start_frame, flags, &mut self.alloc).unwrap().ignore();
        }
    }

    #[inline(always)]
    pub unsafe fn activate(&self) {
        x86_64::registers::control::Cr3::write(
            self.page_table,
            x86_64::registers::control::Cr3Flags::empty(),
        );
        x86_64::instructions::tlb::flush_all();
    }
}
