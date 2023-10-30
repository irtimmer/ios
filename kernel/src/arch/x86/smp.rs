use alloc::alloc;

use core::alloc::Allocator;
use core::alloc::Layout;
use core::hint::spin_loop;
use core::ptr;

use x86_64::VirtAddr;
use x86_64::registers::control::Cr3;

use crate::arch::system::{MemoryFlags, PageMapper};

use super::boot::start_cpu;
use super::lapic::local_apic;
use super::paging::PageTable;

const SMP_BOOT_CODE_ADDRESS: usize = 0x8000;

const SMP_BOOT_CODE_OFFSET_ENTRY: usize = 0x08;
const SMP_BOOT_CODE_OFFSET_CPU_ID: usize = SMP_BOOT_CODE_OFFSET_ENTRY + 0x08;
const SMP_BOOT_CODE_OFFSET_STACK: usize = SMP_BOOT_CODE_OFFSET_CPU_ID + 0x04;
const SMP_BOOT_CODE_OFFSET_PML4: usize = SMP_BOOT_CODE_OFFSET_STACK + 0x08;

static mut DETECTED_CPUS: usize = 0;
static mut BOOTED_CPUS: usize = 0;

pub fn setup_boot_code(memory: &mut PageTable) {
    let boot_code = include_bytes!(concat!(core::env!("OUT_DIR"), "/boot.bin"));
    let boot_code_addr = VirtAddr::new(SMP_BOOT_CODE_ADDRESS as u64);

    unsafe {
        memory.map(SMP_BOOT_CODE_ADDRESS, SMP_BOOT_CODE_ADDRESS, 4096, MemoryFlags::EXECUTABLE | MemoryFlags::WRITABLE).unwrap();
		ptr::copy_nonoverlapping(boot_code.as_ptr(), SMP_BOOT_CODE_ADDRESS as *mut u8, boot_code.len());

		// Pass the PML4 page table address to the boot code.
        *((boot_code_addr + SMP_BOOT_CODE_OFFSET_PML4).as_mut_ptr::<u32>()) = Cr3::read_raw().0.start_address().as_u64() as u32;

        // Pass the entry point to the boot code.
		ptr::write_unaligned((boot_code_addr + SMP_BOOT_CODE_OFFSET_ENTRY).as_mut_ptr(), _start_cpu as usize);
	}
}

pub fn boot_cpu(core_id_to_boot: u32, apic_id: u32) {
    let boot_code = VirtAddr::new(SMP_BOOT_CODE_ADDRESS as u64);

    let stack = alloc::Global::default().allocate(Layout::from_size_align(2048 * 1024, 4096).unwrap()).unwrap();
    let stack_addr = VirtAddr::new(stack.as_ptr() as *const () as u64);
    let stack_top: VirtAddr = stack_addr + (2048_u64 * 1024);

    unsafe {
        ptr::write_unaligned(
			(boot_code + SMP_BOOT_CODE_OFFSET_STACK).as_mut_ptr(),
			stack_top.as_u64(),
		);

        *((boot_code + SMP_BOOT_CODE_OFFSET_CPU_ID).as_mut_ptr()) =
            core_id_to_boot;

        DETECTED_CPUS += 1;
        let lapic = local_apic();
        lapic.send_init_ipi(apic_id as u32);
        lapic.send_sipi((SMP_BOOT_CODE_ADDRESS >> 12) as u8, apic_id as u32);

        while DETECTED_CPUS != BOOTED_CPUS {
            spin_loop();
        }
    }
}

#[inline(never)]
#[no_mangle]
pub unsafe extern "sysv64" fn _start_cpu(cpu_id: u32) -> ! {
    unsafe { BOOTED_CPUS += 1 }

    start_cpu(cpu_id);
}
