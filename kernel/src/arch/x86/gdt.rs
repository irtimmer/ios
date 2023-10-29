use alloc::boxed::Box;

use x86_64::VirtAddr;
use x86_64::instructions::segmentation::Segment;
use x86_64::instructions::tables::load_tss;
use x86_64::registers::segmentation::{CS, DS, SS};
use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;

pub struct Selectors {
    code: SegmentSelector,
    data: SegmentSelector,
    tss: SegmentSelector,
    pub user_data: SegmentSelector,
    pub user_code: SegmentSelector
}

pub fn init() -> Selectors {
    let tss = {
        let mut tss = TaskStateSegment::new();
        tss.privilege_stack_table[0] = {
            const STACK_SIZE: usize = 4096;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });
            let stack_end = stack_start + STACK_SIZE;
            stack_end
        };
        Box::leak(Box::new(tss))
    };

    let (gdt, selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let data_selector = gdt.add_entry(Descriptor::kernel_data_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(tss));
        let user_data_selector = gdt.add_entry(Descriptor::user_data_segment());
        let user_code_selector = gdt.add_entry(Descriptor::user_code_segment());
        (Box::leak(Box::new(gdt)), Selectors {
            code: code_selector,
            data: data_selector,
            tss: tss_selector,
            user_data: user_data_selector,
            user_code: user_code_selector
        })
    };

    gdt.load();
    unsafe {
        CS::set_reg(selectors.code);
        DS::set_reg(selectors.data);
        SS::set_reg(selectors.data);
        load_tss(selectors.tss);
    }

    selectors
}
