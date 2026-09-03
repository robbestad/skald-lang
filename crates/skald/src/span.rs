/// Byte offset range in the source pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        Self {
            start: start as u32,
            end: end as u32,
        }
    }

    pub const fn empty() -> Self {
        Self { start: 0, end: 0 }
    }

    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    pub fn as_range(self) -> std::ops::Range<usize> {
        self.start as usize..self.end as usize
    }
}
