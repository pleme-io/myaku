//! Fixed-size circular buffer for time series data.
//!
//! Used to store metric history for sparkline and chart rendering.
//! Ring buffers have a fixed capacity and overwrite oldest entries when full.

/// A fixed-size circular buffer storing `f32` data points.
#[derive(Debug, Clone)]
pub struct RingBuffer {
    data: Vec<f32>,
    capacity: usize,
    head: usize,
    len: usize,
}

#[allow(dead_code)]
impl RingBuffer {
    /// Create a new ring buffer with the given capacity.
    ///
    /// # Panics
    /// Panics if capacity is zero.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "ring buffer capacity must be > 0");
        Self {
            data: vec![0.0; capacity],
            capacity,
            head: 0,
            len: 0,
        }
    }

    /// Push a new value into the ring buffer, overwriting the oldest if full.
    pub fn push(&mut self, value: f32) {
        self.data[self.head] = value;
        self.head = (self.head + 1) % self.capacity;
        if self.len < self.capacity {
            self.len += 1;
        }
    }

    /// Returns the values in chronological order (oldest first).
    #[must_use]
    pub fn values(&self) -> Vec<f32> {
        if self.len < self.capacity {
            // Buffer not full yet: data starts at index 0
            self.data[..self.len].to_vec()
        } else {
            // Buffer full: head points to the oldest entry
            let mut result = Vec::with_capacity(self.capacity);
            result.extend_from_slice(&self.data[self.head..]);
            result.extend_from_slice(&self.data[..self.head]);
            result
        }
    }

    /// Returns the most recent value, or `None` if empty.
    #[must_use]
    pub fn latest(&self) -> Option<f32> {
        if self.len == 0 {
            None
        } else {
            let idx = if self.head == 0 {
                self.capacity - 1
            } else {
                self.head - 1
            };
            Some(self.data[idx])
        }
    }

    /// Returns the current number of stored values.
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns true if the buffer contains no values.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the buffer capacity.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the minimum value in the buffer, or 0.0 if empty.
    #[must_use]
    pub fn min(&self) -> f32 {
        if self.is_empty() {
            return 0.0;
        }
        self.values()
            .iter()
            .copied()
            .fold(f32::INFINITY, f32::min)
    }

    /// Returns the maximum value in the buffer, or 0.0 if empty.
    #[must_use]
    pub fn max(&self) -> f32 {
        if self.is_empty() {
            return 0.0;
        }
        self.values()
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max)
    }

    /// Returns the average of all values in the buffer, or 0.0 if empty.
    #[must_use]
    pub fn average(&self) -> f32 {
        if self.is_empty() {
            return 0.0;
        }
        let vals = self.values();
        vals.iter().sum::<f32>() / vals.len() as f32
    }

    /// Clear all data.
    pub fn clear(&mut self) {
        self.head = 0;
        self.len = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_buffer_is_empty() {
        let rb = RingBuffer::new(10);
        assert!(rb.is_empty());
        assert_eq!(rb.len(), 0);
        assert_eq!(rb.capacity(), 10);
        assert!(rb.latest().is_none());
    }

    #[test]
    fn push_and_read() {
        let mut rb = RingBuffer::new(5);
        rb.push(1.0);
        rb.push(2.0);
        rb.push(3.0);
        assert_eq!(rb.len(), 3);
        assert_eq!(rb.values(), vec![1.0, 2.0, 3.0]);
        assert_eq!(rb.latest(), Some(3.0));
    }

    #[test]
    fn wraps_around() {
        let mut rb = RingBuffer::new(3);
        rb.push(1.0);
        rb.push(2.0);
        rb.push(3.0);
        rb.push(4.0); // overwrites 1.0
        assert_eq!(rb.len(), 3);
        assert_eq!(rb.values(), vec![2.0, 3.0, 4.0]);
        assert_eq!(rb.latest(), Some(4.0));
    }

    #[test]
    fn min_max_average() {
        let mut rb = RingBuffer::new(5);
        rb.push(10.0);
        rb.push(20.0);
        rb.push(30.0);
        assert!((rb.min() - 10.0).abs() < f32::EPSILON);
        assert!((rb.max() - 30.0).abs() < f32::EPSILON);
        assert!((rb.average() - 20.0).abs() < f32::EPSILON);
    }

    #[test]
    fn empty_min_max_average() {
        let rb = RingBuffer::new(5);
        assert!((rb.min() - 0.0).abs() < f32::EPSILON);
        assert!((rb.max() - 0.0).abs() < f32::EPSILON);
        assert!((rb.average() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn clear_resets() {
        let mut rb = RingBuffer::new(5);
        rb.push(1.0);
        rb.push(2.0);
        rb.clear();
        assert!(rb.is_empty());
        assert_eq!(rb.len(), 0);
    }

    #[test]
    #[should_panic(expected = "capacity must be > 0")]
    fn zero_capacity_panics() {
        let _ = RingBuffer::new(0);
    }
}
