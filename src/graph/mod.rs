//! Graph engine: ring buffers and sparkline data for chart rendering.
//!
//! Stores time-series history for each metric and provides data formatted
//! for GPU chart rendering.

pub mod ring_buffer;

pub use ring_buffer::RingBuffer;

/// Sparkline data: normalized values (0.0-1.0) ready for GPU rendering.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SparklineData {
    /// The label for this sparkline.
    pub label: String,
    /// Normalized values between 0.0 and 1.0.
    pub points: Vec<f32>,
    /// The current (latest) raw value.
    pub current: f32,
    /// Maximum raw value (for display).
    pub max: f32,
    /// Color as RGBA.
    pub color: [f32; 4],
}

impl SparklineData {
    /// Create sparkline data from a ring buffer.
    ///
    /// Values are normalized against `max_value`. If `max_value` is 0.0,
    /// all points are set to 0.0.
    #[must_use]
    pub fn from_ring_buffer(
        buffer: &RingBuffer,
        label: impl Into<String>,
        max_value: f32,
        color: [f32; 4],
    ) -> Self {
        let raw = buffer.values();
        let points = if max_value > 0.0 {
            raw.iter().map(|v| (v / max_value).clamp(0.0, 1.0)).collect()
        } else {
            vec![0.0; raw.len()]
        };

        Self {
            label: label.into(),
            points,
            current: buffer.latest().unwrap_or(0.0),
            max: max_value,
            color,
        }
    }
}

/// A collection of ring buffers for a multi-series chart (e.g. per-core CPU).
#[derive(Debug, Clone)]
pub struct SeriesGroup {
    /// Label for the entire group (e.g. "CPU").
    pub label: String,
    /// Individual series (e.g. one per core).
    pub series: Vec<(String, RingBuffer)>,
    /// A summary ring buffer (e.g. total/average).
    pub summary: RingBuffer,
}

impl SeriesGroup {
    /// Create a new series group.
    #[must_use]
    pub fn new(label: impl Into<String>, count: usize, capacity: usize) -> Self {
        let label = label.into();
        let series = (0..count)
            .map(|i| (format!("{label} {i}"), RingBuffer::new(capacity)))
            .collect();
        Self {
            label,
            series,
            summary: RingBuffer::new(capacity),
        }
    }

    /// Push values for all series and update the summary with their average.
    pub fn push_all(&mut self, values: &[f32]) {
        for (i, (_, buf)) in self.series.iter_mut().enumerate() {
            if let Some(&v) = values.get(i) {
                buf.push(v);
            }
        }
        if !values.is_empty() {
            let avg = values.iter().sum::<f32>() / values.len() as f32;
            self.summary.push(avg);
        }
    }

    /// Resize the series count (e.g. if CPU core count changes).
    pub fn resize(&mut self, count: usize, capacity: usize) {
        while self.series.len() < count {
            let i = self.series.len();
            self.series
                .push((format!("{} {i}", self.label), RingBuffer::new(capacity)));
        }
        self.series.truncate(count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparkline_from_buffer() {
        let mut buf = RingBuffer::new(5);
        buf.push(25.0);
        buf.push(50.0);
        buf.push(75.0);

        let spark = SparklineData::from_ring_buffer(&buf, "test", 100.0, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(spark.label, "test");
        assert_eq!(spark.points.len(), 3);
        assert!((spark.points[0] - 0.25).abs() < f32::EPSILON);
        assert!((spark.points[1] - 0.50).abs() < f32::EPSILON);
        assert!((spark.points[2] - 0.75).abs() < f32::EPSILON);
        assert!((spark.current - 75.0).abs() < f32::EPSILON);
    }

    #[test]
    fn sparkline_zero_max() {
        let mut buf = RingBuffer::new(3);
        buf.push(10.0);
        let spark = SparklineData::from_ring_buffer(&buf, "zero", 0.0, [0.0; 4]);
        assert!((spark.points[0] - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn series_group_push_and_summary() {
        let mut sg = SeriesGroup::new("CPU", 4, 10);
        sg.push_all(&[10.0, 20.0, 30.0, 40.0]);

        assert_eq!(sg.series.len(), 4);
        assert_eq!(sg.series[0].1.latest(), Some(10.0));
        assert_eq!(sg.series[3].1.latest(), Some(40.0));
        assert_eq!(sg.summary.latest(), Some(25.0)); // average
    }

    #[test]
    fn series_group_resize() {
        let mut sg = SeriesGroup::new("Net", 2, 5);
        sg.resize(4, 5);
        assert_eq!(sg.series.len(), 4);
        sg.resize(1, 5);
        assert_eq!(sg.series.len(), 1);
    }
}
