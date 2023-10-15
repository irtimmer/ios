pub trait System {
    fn sleep();
    unsafe fn map(&self, from: usize, to: usize, length: usize) -> Result<(), &'static str>;
}
