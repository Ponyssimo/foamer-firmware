use core::ops::Range;
use heapless::Vec;
use defmt::Format;

#[derive(Format)]
pub struct StringCollection<const N: usize, const K: usize> {
    // Indexes within utf8 bytes
    pairs: [Range<usize>; K],
    heap: Vec<u8, N>,
}

impl<const N: usize, const K: usize> Default for StringCollection<N, K> {
    fn default() -> Self {
        Self {
            pairs: core::array::repeat(Default::default()),
            heap: Default::default(),
        }
    }
}

impl<const N: usize, const K: usize> StringCollection<N, K> {
    pub fn set(&mut self, key: usize, new_str: &str) -> Result<(), heapless::CapacityError> {
        assert!(key < self.pairs.len());
        // Guaranteed sound UTF-8 by `str`
        let new_bytes = new_str.as_bytes();

        if self.heap[self.pairs[key].clone()].len() == new_bytes.len() {
            // Sound because we checked the lengths above, and the utf8 is valid
            // because new_str is an &str
            self.heap[self.pairs[key].clone()].copy_from_slice(new_bytes);
        } else {
            // Different sizes, let's work on it...
            let old_bytes = &self.heap[self.pairs[key].clone()];
            let length_delta: isize = new_bytes.len() as isize - old_bytes.len() as isize;

            if (self.pairs[self.pairs.len() - 1].end as isize + length_delta) as usize > N {
                defmt::error!(
                    "StringCollection: Not enough space to add {} at key {}",
                    new_str,
                    key
                );
                return Err(Default::default());
            }

            // Shift everything after us (and including us) over
            if length_delta > 0 {
                // Adding new characters
                for (index, item) in new_bytes.iter().copied().enumerate().skip(old_bytes.len()) {
                    defmt::unwrap!(self.heap
                        .insert(self.pairs[key].start + index, item));
                }
            } else {
                // Removing old characters
                for _ in new_bytes.len()..old_bytes.len() {
                    // Same number each time because we're removing them
                    self.heap
                        .remove(self.pairs[key].end + (-length_delta).cast_unsigned());
                }
            }

            // Shift indexes too:
            for Range { start, end } in self.pairs.iter_mut().skip(key) {
                *start = (*start as isize + length_delta) as usize;
                *end = (*end as isize + length_delta) as usize;
            }

            // Finally, new data can go in!
            self.heap[self.pairs[key].clone()].copy_from_slice(new_bytes);
        }
        Ok(())
    }

    pub fn get(&self, key: usize) -> &str {
        let bytes = &self.heap[self.pairs[key].clone()];
        unsafe { str::from_utf8_unchecked(bytes) }
    }

    pub fn clear(&mut self, key: usize) {
        defmt::unwrap!(self.set(key, ""));
    }

    pub fn clear_all(&mut self) {
        self.heap.clear();
        self.pairs = core::array::repeat(Default::default());
    }
}
