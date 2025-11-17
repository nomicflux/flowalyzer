use std::collections::VecDeque;

pub(crate) struct ChunkMemory {
    chunks: VecDeque<Vec<f32>>,
    capacity: usize,
}

impl ChunkMemory {
    pub fn new(capacity: usize) -> Self {
        Self {
            chunks: VecDeque::with_capacity(capacity.min(2)),
            capacity: capacity.min(2),
        }
    }

    pub fn remember(&mut self, samples: &[f32]) {
        if self.capacity == 0 {
            return;
        }
        if self.chunks.len() == self.capacity {
            self.chunks.pop_front();
        }
        self.chunks.push_back(samples.to_vec());
    }

    pub fn previous(&self) -> Option<&[f32]> {
        self.chunks.back().map(|chunk| chunk.as_slice())
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    pub fn clear(&mut self) {
        self.chunks.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::ChunkMemory;

    #[test]
    fn remembers_only_one_chunk_when_capacity_is_one() {
        let mut memory = ChunkMemory::new(1);
        memory.remember(&[0.0, 1.0]);
        memory.remember(&[2.0, 3.0]);
        assert_eq!(memory.len(), 1);
        assert_eq!(memory.previous().unwrap(), &[2.0, 3.0]);
    }

    #[test]
    fn caps_capacity_at_two_chunks() {
        let mut memory = ChunkMemory::new(5);
        memory.remember(&[0.0]);
        memory.remember(&[1.0]);
        memory.remember(&[2.0]);
        assert_eq!(memory.len(), 2);
        assert_eq!(memory.previous().unwrap(), &[2.0]);
    }
}
