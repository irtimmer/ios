#![no_main]
#![no_std]

use core::arch::asm;
use core::panic::PanicInfo;

use bootloader::{Framebuffer, BootInfo};

use uefi::{Handle, Status, entry};
use uefi::table::{SystemTable, Boot};
use uefi::table::boot::{AllocateType, MemoryType};
use uefi::proto::console::gop::GraphicsOutput;
use uefi::table::cfg::ACPI2_GUID;

use x86_64::instructions;

use xmas_elf::ElfFile;
use xmas_elf::dynamic::Tag;
use xmas_elf::program::{Type, SegmentData};
use xmas_elf::sections::Rela;

#[macro_export]
macro_rules! include_bytes_aligned {
    ($path:expr) => {{
        #[repr(align(32))]
        pub struct Aligned32;

        #[repr(C)]
        pub struct Aligned<Bytes: ?Sized> {
            pub _align: [Aligned32; 0],
            pub bytes: Bytes,
        }

        static ALIGNED: &Aligned<[u8]> = &Aligned {
            _align: [],
            bytes: *include_bytes!($path),
        };

        &ALIGNED.bytes
    }};
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop { instructions::hlt(); }
}

#[entry]
#[allow(named_asm_labels)]
fn main(_handle: Handle, system_table: SystemTable<Boot>) -> Status {
    let fb = {
        let bt = system_table.boot_services();
        let gop_handle = bt.get_handle_for_protocol::<GraphicsOutput>().unwrap();
        let mut gop = bt.open_protocol_exclusive::<GraphicsOutput>(gop_handle).unwrap();
        Framebuffer {
            width: gop.current_mode_info().resolution().0,
            height: gop.current_mode_info().resolution().1,
            stride: gop.current_mode_info().stride(),
            bpp: 32,
            buffer: gop.frame_buffer().as_mut_ptr()
        }
    };

    // Get location of ACPI tables
    let cfg_tbl = system_table.config_table();
    let acpi_table = cfg_tbl.iter().find(|entry| entry.guid == ACPI2_GUID).map(|entry| entry.address as usize);

    let kernel = include_bytes_aligned!("../../target/x86_64-unknown-none/debug/ios");
    let elf = ElfFile::new(kernel).unwrap();
    let entry_point = elf.header.pt2.entry_point() as usize;

    let size = elf.program_iter().filter(|x| x.get_type().unwrap() == Type::Load).map(|x| x.virtual_addr() + x.mem_size()).max().unwrap();
    let pages = (size / 4096 + 1) as usize;

    let kernel_address = system_table.boot_services().allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_CODE, pages).unwrap();
    unsafe {
        let kernel = core::slice::from_raw_parts_mut(kernel_address as *mut u8, size as usize);
        kernel.fill(0);
    }
    let kernel_pointer = kernel_address as *const u8;
    for program in elf.program_iter() {
        match program.get_type().unwrap() {
            Type::Load => {
                let segment_address = (kernel_address + program.virtual_addr()) as usize;
                let segment_size = program.file_size() as usize;
                let segment_offset = program.offset() as usize;

                unsafe {
                    let segment = core::slice::from_raw_parts_mut(segment_address as *mut u8, segment_size);
                    segment.copy_from_slice(&kernel[segment_offset..segment_offset + segment_size]);
                }
            },
            Type::Dynamic => {
                let segment_data = program.get_data(&elf).unwrap();
                let segment_data = if let SegmentData::Dynamic64(segment_data) = segment_data {
                    segment_data
                } else {
                    panic!("expected Dynamic64 segment")
                };
                let rela = segment_data.iter().find(|x| x.get_tag().unwrap() == Tag::Rela).unwrap();
                let rela_size = segment_data.iter().find(|x| x.get_tag().unwrap() == Tag::RelaSize).unwrap();
                let rela_ent = segment_data.iter().find(|x| x.get_tag().unwrap() == Tag::RelaEnt).unwrap();

                let total_size = rela_size.get_val().unwrap();
                let entry_size = rela_ent.get_val().unwrap();
                let num_entries = total_size / entry_size;
                let offset = rela.get_ptr().unwrap();

                let relas: &[Rela<u64>] = unsafe {
                    core::slice::from_raw_parts(kernel.as_ptr().wrapping_add(offset as usize) as *const Rela<u64>, num_entries as usize)
                };
                for rela in relas {
                    match rela.get_type() {
                        8 => {
                            let ptr = kernel_address.wrapping_add(rela.get_offset()) as *mut u64;
                            unsafe { *ptr = kernel_address.wrapping_add(rela.get_addend()) };
                        },
                        _ => panic!("Unsupported relocation type: {}", rela.get_type())
                    }
                }
            },
            _ => {}
        }
    }

    // Exit UEFI boot services
    let (_, memory_map) = system_table.exit_boot_services();

    unsafe {
        let info = BootInfo {
            framebuffer: fb,
            acpi_table: acpi_table,
            memory_map: memory_map,
        };

        asm!("jmp {}", in(reg) kernel_pointer.wrapping_add(entry_point), in("rdi") &info as *const _ as u64);
    }

    Status::ABORTED
}
