pub struct FrameBuffer {
    pub width: usize,
    pub height: usize,
    pub stride: usize,
    pub bpp: usize,
    pub buffer: *mut u8,
}
