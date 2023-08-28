#![allow(unused)]
// offer an intermediate buffer (or iterator)
// that represents the sound to play in the future

use std::cmp::min;

pub struct RingBuffer<T> {
    capacity: usize,
    buffer: Vec<T>,
    // start of data, including
    head: usize,
    // end of data, excluding
    tail: usize,
    len: usize,
}

impl<T: Default> RingBuffer<T> {
    pub fn with_default(capacity: usize) -> Self {
        Self::new(capacity, Default::default)
    }
}

impl<T> RingBuffer<T> {
    pub fn new<F>(capacity: usize, mut fill: F) -> Self
    where
        F: FnMut() -> T,
    {
        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(fill());
        }

        Self {
            capacity,
            buffer,
            head: 0,
            tail: 0,
            len: 0,
        }
    }

    pub fn space(&self) -> usize {
        self.capacity - self.len()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn write_buffers(&mut self, amount: usize) -> (&mut [T], &mut [T]) {
        if self.len == self.capacity {
            return (&mut [], &mut []);
        }

        let space = self.space();
        let amount = min(space, amount);
        self.len += amount;

        if self.tail >= self.head {
            let (right, left) = self.buffer.split_at_mut(self.tail);
            let left_len = left.len();

            let left = &mut left[0..min(amount, left_len)];

            let remainder = amount - left.len();

            let right = &mut right[0..remainder];

            self.tail += amount;
            self.tail %= self.capacity;

            (left, right)
        } else {
            let left = &mut self.buffer[self.tail..min(amount + self.tail, self.head)];

            self.tail += left.len();
            self.tail %= self.capacity;

            (left, &mut [])
        }
    }

    pub fn read(&mut self, amount: usize) -> (&[T], &[T]) {
        if self.len == 0 {
            return (&[], &[]);
        }
        let amount = min(self.len, amount);

        self.len -= amount;

        if self.tail <= self.head {
            let left = &self.buffer[self.head..min(amount + self.head, self.capacity)];
            let remainder = amount - left.len();
            let right = &self.buffer[0..min(remainder, self.tail)];

            self.head += left.len() + right.len();
            self.head %= self.capacity;

            (left, right)
        } else {
            let left = &self.buffer[self.head..min(amount + self.head, self.tail)];

            self.head += left.len();
            self.head %= self.capacity;

            (left, &[])
        }
    }
}

#[cfg(test)]
mod ring_buffer_tests {

    //TODO(voided): rewrite tests in term of write and reads and try to avoid messing with internal data

    use crate::utility::ring_buffer::RingBuffer;

    #[test]
    fn empty_len() {
        let mut buffer: RingBuffer<i32> = RingBuffer::with_default(10);
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn empty_len_middle() {
        let mut buffer: RingBuffer<i32> = RingBuffer {
            capacity: 10,
            buffer: vec![],
            head: 5,
            tail: 5,
            len: 0,
        };

        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn len_tail_before_head() {
        let mut buffer: RingBuffer<i32> = RingBuffer {
            capacity: 10,
            buffer: vec![],
            head: 5,
            tail: 4,
            len: 9,
        };

        assert_eq!(buffer.len(), 9);
    }

    #[test]
    fn len_head_before_tail() {
        let mut buffer: RingBuffer<i32> = RingBuffer {
            capacity: 10,
            buffer: vec![],
            head: 1,
            tail: 5,
            len: 4,
        };

        assert_eq!(buffer.len(), 4);
    }

    #[test]
    fn empty_space() {
        let mut buffer: RingBuffer<i32> = RingBuffer::with_default(10);
        assert_eq!(buffer.space(), 10);
    }

    #[test]
    fn empty_space_middle() {
        let mut buffer: RingBuffer<i32> = RingBuffer {
            capacity: 10,
            buffer: vec![],
            head: 5,
            tail: 5,
            len: 0,
        };

        assert_eq!(buffer.space(), 10);
    }

    #[test]
    fn space_tail_before_head() {
        let mut buffer: RingBuffer<i32> = RingBuffer {
            capacity: 10,
            buffer: vec![],
            head: 5,
            tail: 3,
            len: 9,
        };

        assert_eq!(buffer.space(), 1);
    }

    #[test]
    fn space_head_before_tail() {
        let mut buffer: RingBuffer<i32> = RingBuffer {
            capacity: 10,
            buffer: vec![],
            head: 1,
            tail: 5,
            len: 4,
        };

        assert_eq!(buffer.space(), 6);
    }

    #[test]
    fn write_empty_buffer_sizes() {
        let mut buffer: RingBuffer<i32> = RingBuffer::with_default(10);
        let (l, r) = buffer.write_buffers(5);
        assert_eq!((l.len(), r.len()), (5, 0));
    }

    #[test]
    fn write_empty() {
        let mut buffer: RingBuffer<i32> = RingBuffer::with_default(10);
        let (l, r) = buffer.write_buffers(5);
        for v in l.iter_mut() {
            *v = 1;
        }
        for v in r.iter_mut() {
            *v = 2;
        }
        assert_eq!(buffer.buffer, Vec::from([1, 1, 1, 1, 1, 0, 0, 0, 0, 0]));
    }

    #[test]
    fn write_empty_middle() {
        let mut buffer: RingBuffer<i32> = RingBuffer {
            capacity: 10,
            buffer: vec![0; 10],
            head: 5,
            tail: 5,
            len: 0,
        };
        let (l, r) = buffer.write_buffers(5);
        for v in l.iter_mut() {
            *v = 1;
        }
        for v in r.iter_mut() {
            *v = 2;
        }
        assert_eq!(buffer.buffer, Vec::from([0, 0, 0, 0, 0, 1, 1, 1, 1, 1]));
    }

    #[test]
    fn write_empty_border() {
        let mut buffer: RingBuffer<i32> = RingBuffer {
            capacity: 10,
            buffer: vec![0; 10],
            head: 7,
            tail: 7,
            len: 0,
        };
        let (l, r) = buffer.write_buffers(5);
        for v in l.iter_mut() {
            *v = 1;
        }
        for v in r.iter_mut() {
            *v = 2;
        }
        assert_eq!(buffer.buffer, Vec::from([2, 2, 0, 0, 0, 0, 0, 1, 1, 1]));
    }

    #[test]
    fn write_empty_too_much() {
        let mut buffer: RingBuffer<i32> = RingBuffer {
            capacity: 10,
            buffer: vec![0; 11],
            head: 7,
            tail: 7,
            len: 0,
        };
        let (l, r) = buffer.write_buffers(15);
        for v in l.iter_mut() {
            *v = 1;
        }
        for v in r.iter_mut() {
            *v = 2;
        }
        assert_eq!(buffer.buffer, Vec::from([2, 2, 2, 2, 2, 2, 0, 1, 1, 1, 1]));
    }

    #[test]
    fn write_full() {
        let mut buffer: RingBuffer<i32> = RingBuffer {
            capacity: 10,
            buffer: vec![0; 10],
            head: 7,
            tail: 6,
            len: 10,
        };
        let (l, r) = buffer.write_buffers(15);
        for v in l.iter_mut() {
            *v = 1;
        }
        for v in r.iter_mut() {
            *v = 2;
        }
        assert_eq!(buffer.buffer, Vec::from([0, 0, 0, 0, 0, 0, 0, 0, 0, 0]));
    }

    #[test]
    fn write_tail_before_head() {
        let mut buffer: RingBuffer<i32> = RingBuffer {
            capacity: 10,
            buffer: vec![0; 10],
            head: 7,
            tail: 2,
            len: 5,
        };
        let (l, r) = buffer.write_buffers(15);
        for v in l.iter_mut() {
            *v = 1;
        }
        for v in r.iter_mut() {
            *v = 2;
        }
        assert_eq!(buffer.buffer, Vec::from([0, 0, 1, 1, 1, 1, 1, 0, 0, 0]));
    }

    #[test]
    fn write_tail_after_head() {
        let mut buffer: RingBuffer<i32> = RingBuffer {
            capacity: 10,
            buffer: vec![0; 10],
            head: 7,
            tail: 9,
            len: 2,
        };
        let (l, r) = buffer.write_buffers(15);
        for v in l.iter_mut() {
            *v = 1;
        }
        for v in r.iter_mut() {
            *v = 2;
        }
        assert_eq!(buffer.buffer, Vec::from([2, 2, 2, 2, 2, 2, 2, 0, 0, 1]));
    }

    #[test]
    fn read_empty() {
        let mut buffer: RingBuffer<i16> = RingBuffer::with_default(5);
        let (l, r) = buffer.read(5);
        assert_eq!((l.len(), r.len()), (0, 0));
    }

    #[test]
    fn read_2() {
        let mut buffer: RingBuffer<i16> = RingBuffer::with_default(5);

        buffer.write_buffers(2);

        let (l, r) = buffer.read(5);
        assert_eq!((l.len(), r.len()), (2, 0));
    }

    #[test]
    fn test() {
        let mut buffer: RingBuffer<i16> = RingBuffer::with_default(10);
        let (l, r) = buffer.write_buffers(2);
        l.fill(1);
        r.fill(-1);
        let (l, r) = buffer.write_buffers(5);
        l.fill(2);
        r.fill(-2);
        let (l, r) = buffer.write_buffers(2);
        l.fill(3);
        r.fill(-3);
        assert_eq!(buffer.buffer, Vec::from([1, 1, 2, 2, 2, 2, 2, 3, 3, 0]));

        let (l, r) = buffer.read(5);
        assert_eq!(Vec::from(l), [1, 1, 2, 2, 2]);

        let (l, r) = buffer.write_buffers(4);
        l.fill(4);
        r.fill(-4);

        println!("{}, {}, {}", buffer.len, buffer.head, buffer.tail);

        let (l, r) = buffer.read(9);
        assert_eq!(Vec::from(l), [2, 2, 3, 3, 4]);
        assert_eq!(Vec::from(r), [-4, -4, -4]);
    }
}
