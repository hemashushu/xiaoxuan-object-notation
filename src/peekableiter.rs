// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

/// Unlike `std::iter::Peekable`, PeekableIter
/// supports peek to specify offsets.
pub struct PeekableIter<'a, T>
where
    T: PartialEq,
{
    upstream: &'a mut dyn Iterator<Item = T>,
    buffer: RoundQueue<T>,
    buffer_size: usize,
}

pub const MAX_LOOKAHEAD_LENGTH: usize = 6;

struct RoundQueue<T>
where
    T: PartialEq,
{
    size: usize,
    position_read: usize,
    position_write: usize,
    data: [Option<T>; MAX_LOOKAHEAD_LENGTH],
}

impl<T> RoundQueue<T>
where
    T: PartialEq,
{
    pub fn new(size: usize) -> Self {
        assert!(size < MAX_LOOKAHEAD_LENGTH);

        Self {
            size,
            position_read: 0,
            position_write: 0,
            data: [None, None, None, None, None, None],
        }
    }

    pub fn enqueue(&mut self, value: Option<T>) {
        self.data[self.position_write] = value;

        self.position_write += 1;
        if self.position_write == self.size {
            self.position_write = 0;
        }
    }

    pub fn dequeue(&mut self) -> Option<T> {
        let value = self.data[self.position_read].take();

        self.position_read += 1;
        if self.position_read == self.size {
            self.position_read = 0;
        }

        value
    }

    pub fn peek(&self, offset: usize) -> Option<&T> {
        assert!(offset < self.size);

        let mut position = self.position_read + offset;
        if position >= self.size {
            position -= self.size;
        }

        self.data[position].as_ref()
    }
}

impl<'a, T> PeekableIter<'a, T>
where
    T: PartialEq,
{
    pub fn new(upstream: &'a mut dyn Iterator<Item = T>, buffer_size: usize) -> Self {
        let mut buffer = RoundQueue::new(buffer_size);

        // pre-fill
        for _ in 0..buffer_size {
            let value = upstream.next();
            buffer.enqueue(value);
        }

        Self {
            upstream,
            buffer,
            buffer_size,
        }
    }

    pub fn peek(&self, offset: usize) -> Option<&T> {
        assert!(offset < self.buffer_size);
        self.buffer.peek(offset)
    }
}

impl<T> Iterator for PeekableIter<'_, T>
where
    T: PartialEq,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        // take the data from buffer first, and then insert the new data,
        // because the buffer is pre-filled when it is created.
        let value = self.buffer.dequeue();

        let next_value = self.upstream.next();
        self.buffer.enqueue(next_value);

        value
    }
}

#[cfg(test)]
mod tests {
    use crate::peekableiter::PeekableIter;

    #[test]
    fn test_peekable_iter() {
        let s = "0123";
        let mut chars = s.chars();
        let mut iter = PeekableIter::new(&mut chars, 3);

        // init
        assert_eq!(Some(&'0'), iter.peek(0));
        assert_eq!(Some(&'1'), iter.peek(1));
        assert_eq!(Some(&'2'), iter.peek(2));

        // consume '0'
        assert_eq!(Some('0'), iter.next());
        assert_eq!(Some(&'1'), iter.peek(0));
        assert_eq!(Some(&'2'), iter.peek(1));
        assert_eq!(Some(&'3'), iter.peek(2));

        // consume '1'
        assert_eq!(Some('1'), iter.next());
        assert_eq!(Some(&'2'), iter.peek(0));
        assert_eq!(Some(&'3'), iter.peek(1));
        assert_eq!(None, iter.peek(2));

        // consume '2'
        assert_eq!(Some('2'), iter.next());
        assert_eq!(Some(&'3'), iter.peek(0));
        assert_eq!(None, iter.peek(1));
        assert_eq!(None, iter.peek(2));

        // consume '3'
        assert_eq!(Some('3'), iter.next());
        assert_eq!(None, iter.peek(0));
        assert_eq!(None, iter.peek(1));
        assert_eq!(None, iter.peek(2));

        // empty
        assert_eq!(None, iter.next());
        assert_eq!(None, iter.peek(0));
        assert_eq!(None, iter.peek(1));
        assert_eq!(None, iter.peek(2));
    }

    #[test]
    fn test_nested_peekable_iter() {
        let s = "0123";
        let mut chars = s.chars();
        let mut iter1 = PeekableIter::new(&mut chars, 3);
        let mut iter2 = PeekableIter::new(&mut iter1, 3);

        // init
        assert_eq!(Some(&'0'), iter2.peek(0));
        assert_eq!(Some(&'1'), iter2.peek(1));
        assert_eq!(Some(&'2'), iter2.peek(2));

        // consume '0'
        assert_eq!(Some('0'), iter2.next());
        assert_eq!(Some(&'1'), iter2.peek(0));

        // consume '1'
        assert_eq!(Some('1'), iter2.next());
        assert_eq!(Some(&'2'), iter2.peek(0));

        // consume '2'
        assert_eq!(Some('2'), iter2.next());
        assert_eq!(Some(&'3'), iter2.peek(0));

        // consume '3'
        assert_eq!(Some('3'), iter2.next());
        assert_eq!(None, iter2.peek(0));

        // empty
        assert_eq!(None, iter2.next());
        assert_eq!(None, iter2.peek(0));
    }
}
