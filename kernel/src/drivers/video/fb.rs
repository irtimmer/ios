pub struct FrameBuffer {
    pub width: usize,
    pub height: usize,
    pub stride: usize,
    pub bpp: usize,
    pub buffer: *mut u8,
}

// Make *mut u8 thread safe
unsafe impl Send for FrameBuffer {}
unsafe impl Sync for FrameBuffer {}
