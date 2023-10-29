use alloc::alloc;

use core::alloc::Layout;

use x86_64::structures::paging::{FrameAllocator, Mapper, OffsetPageTable, Page, PageSize, PageTable as NativePageTable, PageTableFlags, PhysFrame, Size4KiB};
use x86_64::{PhysAddr, VirtAddr};

use crate::arch::system::{MemoryMapError, MemoryFlags, PageMapper};

#[derive(Default, Clone)]
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

pub struct PageTable {
    alloc: OffsetAllocatorFrameAllocation,
    page_table: PhysFrame<Size4KiB>,
    mapper: OffsetPageTable<'static>,
}

impl PageTable {
    pub fn new(offset: u64) -> Self {
        let mut alloc = OffsetAllocatorFrameAllocation {
            offset: offset as usize,
        };
        let page_frame: PhysFrame<Size4KiB> = alloc.allocate_frame().unwrap();
        let ptr = VirtAddr::new(page_frame.start_address().as_u64() + offset).as_mut_ptr();
        unsafe { *ptr = NativePageTable::new() };
        let page_table = unsafe { &mut *ptr };

        let mapper = unsafe { OffsetPageTable::new(page_table, VirtAddr::new(offset)) };

        Self {
            alloc,
            page_table: page_frame,
            mapper,
        }
    }

    pub unsafe fn clone(&self) -> Self {
        let mut alloc = self.alloc.clone();
        let page_frame: PhysFrame<Size4KiB> = alloc.allocate_frame().unwrap();
        let ptr: *mut NativePageTable = VirtAddr::new(page_frame.start_address().as_u64() + self.alloc.offset as u64).as_mut_ptr();
        let kernel_ptr: *const NativePageTable = VirtAddr::new(self.page_table.start_address().as_u64() + self.alloc.offset as u64).as_mut_ptr();
        unsafe { ptr.copy_from(kernel_ptr, 1) };
        let page_table = unsafe { &mut *ptr };

        let mapper = unsafe { OffsetPageTable::new(page_table, VirtAddr::new(self.alloc.offset as u64)) };

        Self {
            alloc,
            page_table: page_frame,
            mapper
        }
    }
}

impl PageMapper for PageTable {
    unsafe fn map(&mut self, from: usize, to: usize, length: usize, map_flags: MemoryFlags) -> Result<(), MemoryMapError> {
        let mut flags = PageTableFlags::PRESENT;
        if !map_flags.contains(MemoryFlags::EXECUTABLE) {
            flags |= PageTableFlags::NO_EXECUTE;
        }

        if map_flags.contains(MemoryFlags::WRITABLE) {
            flags |= PageTableFlags::WRITABLE;
        }

        if map_flags.contains(MemoryFlags::USER) {
            flags |= PageTableFlags::USER_ACCESSIBLE;
        }

        for i in 0..((length as u64) / Size4KiB::SIZE) {
            let start_address = from as u64 + i * Size4KiB::SIZE as u64;
            let start_frame = PhysFrame::<Size4KiB>::from_start_address(PhysAddr::new(start_address)).map_err(|_| MemoryMapError::InvalidAlignment(start_address))?;
            let map_address = to as u64 + i * Size4KiB::SIZE;
            let map_frame = Page::from_start_address(VirtAddr::new(map_address)).map_err(|_| MemoryMapError::InvalidAlignment(start_address))?;
            self.mapper.map_to(map_frame, start_frame, flags, &mut self.alloc).map_err(|_| MemoryMapError::AlreadyMapped(map_address, start_address))?.ignore();
        }
        Ok(())
    }

    #[inline(always)]
    unsafe fn activate(&self) {
        x86_64::registers::control::Cr3::write(
            self.page_table,
            x86_64::registers::control::Cr3Flags::empty(),
        );
        x86_64::instructions::tlb::flush_all();
    }
}
