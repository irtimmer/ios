use alloc::boxed::Box;

use x86_64::instructions::segmentation::Segment;
use x86_64::registers::segmentation::{CS, DS, SS};
use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor, SegmentSelector};

struct Selectors {
    code: SegmentSelector,
    data: SegmentSelector,
}

pub fn init() {
    let (gdt, selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let data_selector = gdt.add_entry(Descriptor::kernel_data_segment());
        (Box::leak(Box::new(gdt)), Selectors { code: code_selector, data: data_selector })
    };

    gdt.load();
    unsafe {
        CS::set_reg(selectors.code);
        DS::set_reg(selectors.data);
        SS::set_reg(selectors.data);
    }
}
