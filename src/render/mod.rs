//! GPU rendering for the myaku system monitor.
//!
//! Implements `madori::RenderCallback` to draw system metrics using garasu
//! text rendering and wgpu primitives.

use egaku::Theme;
use garasu::GpuContext;
use glyphon::{Color as GlyphColor, TextArea};
use madori::render::{RenderCallback, RenderContext};

use crate::graph::SparklineData;
use crate::input::Mode;
use crate::metrics::MetricsCollector;
use crate::process::ProcessList;

/// Convert [f32; 4] RGBA to glyphon Color.
fn to_glyph_color(c: &[f32; 4]) -> GlyphColor {
    GlyphColor::rgba(
        (c[0] * 255.0) as u8,
        (c[1] * 255.0) as u8,
        (c[2] * 255.0) as u8,
        (c[3] * 255.0) as u8,
    )
}

/// The main renderer for myaku.
pub struct MyakuRenderer {
    /// Current theme colors.
    pub theme: Theme,
    /// Font size for body text.
    pub font_size: f32,
    /// Line height.
    pub line_height: f32,
    /// Current render state (set externally before each frame).
    pub state: RenderState,
}

/// Snapshot of data needed for rendering a single frame.
/// Built from `MetricsCollector` + `ProcessList` + `Mode` each frame.
pub struct RenderState {
    /// Lines to display in the dashboard view.
    pub dashboard_lines: Vec<(String, [f32; 4])>,
    /// Lines to display in the process view.
    pub process_lines: Vec<(String, [f32; 4])>,
    /// Current mode.
    pub mode: Mode,
    /// Status bar text.
    pub status_line: String,
    /// Sparkline data for dashboard charts.
    pub sparklines: Vec<SparklineData>,
}

impl Default for RenderState {
    fn default() -> Self {
        Self {
            dashboard_lines: Vec::new(),
            process_lines: Vec::new(),
            mode: Mode::Dashboard,
            status_line: String::new(),
            sparklines: Vec::new(),
        }
    }
}

impl MyakuRenderer {
    /// Create a new renderer with the given theme.
    #[must_use]
    pub fn new(theme: Theme) -> Self {
        let font_size = theme.font_size;
        Self {
            theme,
            font_size,
            line_height: font_size * 1.6,
            state: RenderState::default(),
        }
    }

    /// Build the dashboard render state from live metrics.
    pub fn build_dashboard_state(
        &self,
        collector: &MetricsCollector,
    ) -> Vec<(String, [f32; 4])> {
        let theme = &self.theme;
        let mut lines = Vec::new();

        // Title
        lines.push((
            "  MYAKU - System Monitor".to_string(),
            theme.accent,
        ));
        lines.push((String::new(), theme.foreground));

        // CPU section
        let cpu = &collector.cpu;
        lines.push((
            format!(
                "  CPU: {:.1}%  ({}, {} cores)",
                cpu.total_usage(),
                cpu.brand,
                cpu.core_count
            ),
            self.percent_color(cpu.total_usage()),
        ));

        // Per-core sparklines as compact text bars
        let per_core = cpu.cores.series.iter().enumerate();
        for (i, (_, buf)) in per_core {
            let usage = buf.latest().unwrap_or(0.0);
            let bar = self.text_bar(usage, 100.0, 20);
            lines.push((
                format!("    Core {i:>2}: {bar} {usage:>5.1}%"),
                self.percent_color(usage),
            ));
        }
        lines.push((String::new(), theme.foreground));

        // Memory section
        let mem = &collector.memory;
        lines.push((
            format!(
                "  RAM: {:.1}%  {}/{}",
                mem.ram_percent(),
                mem.used_ram_display(),
                mem.total_ram_display()
            ),
            self.percent_color(mem.ram_percent()),
        ));
        let ram_bar = self.text_bar(mem.ram_percent(), 100.0, 30);
        lines.push((format!("    {ram_bar}"), self.percent_color(mem.ram_percent())));

        if mem.latest.swap_total > 0 {
            lines.push((
                format!(
                    "  Swap: {:.1}%  {}/{}",
                    mem.swap_percent(),
                    mem.used_swap_display(),
                    mem.total_swap_display()
                ),
                self.percent_color(mem.swap_percent()),
            ));
        }
        lines.push((String::new(), theme.foreground));

        // Disk section
        lines.push(("  Disks:".to_string(), theme.foreground));
        for mount in &collector.disk.mounts {
            let pct = mount.usage_percent();
            let bar = self.text_bar(pct, 100.0, 15);
            lines.push((
                format!(
                    "    {} {bar} {:.0}% ({}/{})",
                    mount.mount_point,
                    pct,
                    mount.used_display(),
                    mount.total_display()
                ),
                self.percent_color(pct),
            ));
        }
        lines.push((String::new(), theme.foreground));

        // Network section
        lines.push(("  Network:".to_string(), theme.foreground));
        for iface in &collector.network.interfaces {
            if iface.total_rx == 0 && iface.total_tx == 0 {
                continue;
            }
            lines.push((
                format!(
                    "    {}: rx {} tx {}",
                    iface.name,
                    iface.rx_display(),
                    iface.tx_display()
                ),
                theme.base0c, // cyan for network
            ));
        }
        lines.push((String::new(), theme.foreground));

        // System info
        lines.push((
            format!("  Uptime: {}  Load: {}", collector.uptime_display(), collector.load_display()),
            theme.muted,
        ));

        lines
    }

    /// Build the process view render state.
    pub fn build_process_state(
        &self,
        proc_list: &ProcessList,
    ) -> Vec<(String, [f32; 4])> {
        let theme = &self.theme;
        let mut lines = Vec::new();

        // Title
        lines.push((
            format!(
                "  Processes ({}/{})  Sort: {} {}  Filter: {}",
                proc_list.filtered_count(),
                proc_list.total_count(),
                proc_list.sort_column.label(),
                if proc_list.sort_ascending { "^" } else { "v" },
                if proc_list.filter.is_empty() {
                    "-"
                } else {
                    &proc_list.filter
                },
            ),
            theme.accent,
        ));
        lines.push((String::new(), theme.foreground));

        // Header
        lines.push((format!("  {}", proc_list.header_row()), theme.base04));

        // Process rows
        let visible = proc_list.visible_processes();
        for (i, proc) in visible.iter().enumerate() {
            let row_idx = proc_list.scroll_offset + i;
            let is_selected = row_idx == proc_list.selected;
            let color = if is_selected {
                theme.accent
            } else {
                theme.foreground
            };
            let prefix = if is_selected { "> " } else { "  " };
            lines.push((
                format!("{prefix}{}", ProcessList::format_row(proc)),
                color,
            ));
        }

        lines
    }

    /// Build the sparklines data from metrics.
    pub fn build_sparklines(&self, collector: &MetricsCollector) -> Vec<SparklineData> {
        let mut sparklines = Vec::new();
        sparklines.push(collector.cpu.total_sparkline(self.theme.accent));
        sparklines.push(collector.memory.ram_sparkline(self.theme.base0b));
        sparklines
    }

    /// Generate a text-based progress bar.
    fn text_bar(&self, value: f32, max: f32, width: usize) -> String {
        let ratio = if max > 0.0 {
            (value / max).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let filled = (ratio * width as f32).round() as usize;
        let empty = width.saturating_sub(filled);
        format!("[{}{}]", "#".repeat(filled), "-".repeat(empty))
    }

    /// Color based on usage percentage: green < 50, yellow < 80, red >= 80.
    fn percent_color(&self, pct: f32) -> [f32; 4] {
        if pct >= 80.0 {
            self.theme.error
        } else if pct >= 50.0 {
            self.theme.warning
        } else {
            self.theme.success
        }
    }
}

impl RenderCallback for MyakuRenderer {
    fn init(&mut self, _gpu: &GpuContext) {
        tracing::info!("myaku renderer initialized");
    }

    fn resize(&mut self, _width: u32, _height: u32) {
        // Layout recalculation handled in render() based on ctx dimensions.
    }

    fn render(&mut self, ctx: &mut RenderContext<'_>) {
        // Clear background
        let bg = &self.theme.background;
        let clear_color = wgpu::Color {
            r: f64::from(bg[0]),
            g: f64::from(bg[1]),
            b: f64::from(bg[2]),
            a: f64::from(bg[3]),
        };

        // Choose which lines to render based on mode
        let lines: &[(String, [f32; 4])] = match self.state.mode {
            Mode::Dashboard => &self.state.dashboard_lines,
            Mode::Process | Mode::Filter => &self.state.process_lines,
        };

        // Render sparklines as text between sections
        let owned_spark: Vec<(String, [f32; 4])> = self.render_sparklines_as_text();

        // Build text buffers for all lines
        let mut text_areas = Vec::new();
        let mut buffers = Vec::new();
        let mut y_pos = 4.0_f32;
        let padding_left = 8.0_f32;

        // Render main content lines
        for (text, color) in lines {
            if text.is_empty() {
                y_pos += self.line_height * 0.5;
                continue;
            }
            let mut buffer = ctx.text.create_buffer(
                text,
                self.font_size,
                self.line_height,
            );
            buffer.set_size(
                &mut ctx.text.font_system,
                Some(ctx.width as f32 - padding_left * 2.0),
                Some(self.line_height),
            );
            buffer.shape_until_scroll(&mut ctx.text.font_system, false);
            buffers.push((buffer, *color, y_pos));
            y_pos += self.line_height;
        }

        // Render sparklines below main content
        if self.state.mode == Mode::Dashboard && !owned_spark.is_empty() {
            y_pos += self.line_height * 0.5;
            for (text, color) in &owned_spark {
                if text.is_empty() {
                    y_pos += self.line_height * 0.5;
                    continue;
                }
                let mut buffer = ctx.text.create_buffer(
                    text,
                    self.font_size * 0.85,
                    self.line_height * 0.85,
                );
                buffer.set_size(
                    &mut ctx.text.font_system,
                    Some(ctx.width as f32 - padding_left * 2.0),
                    Some(self.line_height * 0.85),
                );
                buffer.shape_until_scroll(&mut ctx.text.font_system, false);
                buffers.push((buffer, *color, y_pos));
                y_pos += self.line_height * 0.85;
            }
        }

        // Render status bar at the bottom
        if !self.state.status_line.is_empty() {
            let status_y = ctx.height as f32 - self.line_height - 4.0;
            let mut buffer = ctx.text.create_buffer(
                &self.state.status_line,
                self.font_size * 0.9,
                self.line_height * 0.9,
            );
            buffer.set_size(
                &mut ctx.text.font_system,
                Some(ctx.width as f32 - padding_left * 2.0),
                Some(self.line_height * 0.9),
            );
            buffer.shape_until_scroll(&mut ctx.text.font_system, false);
            buffers.push((buffer, self.theme.muted, status_y));
        }

        // Build text areas from buffers
        for (buffer, color, y) in &buffers {
            text_areas.push(TextArea {
                buffer,
                left: padding_left,
                top: *y,
                scale: 1.0,
                bounds: glyphon::TextBounds {
                    left: padding_left as i32,
                    top: *y as i32,
                    right: ctx.width as i32,
                    bottom: (*y + self.line_height * 2.0) as i32,
                },
                default_color: to_glyph_color(color),
                custom_glyphs: &[],
            });
        }

        // Prepare and render text
        ctx.text
            .prepare(
                &ctx.gpu.device,
                &ctx.gpu.queue,
                ctx.width,
                ctx.height,
                text_areas,
            )
            .ok();

        let mut encoder = ctx.gpu.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("myaku_render"),
            },
        );

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("myaku_clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: ctx.surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            ctx.text.render(&mut pass).ok();
        }

        ctx.gpu.queue.submit(std::iter::once(encoder.finish()));
    }
}

impl MyakuRenderer {
    /// Render sparklines as text-art lines.
    fn render_sparklines_as_text(&self) -> Vec<(String, [f32; 4])> {
        let mut lines = Vec::new();
        // Unicode block characters for sparkline: " ", "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"
        let blocks = [' ', '\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}', '\u{2588}'];

        for spark in &self.state.sparklines {
            if spark.points.is_empty() {
                continue;
            }
            // Take last 60 points (or fewer) to fit in a line
            let points: Vec<f32> = if spark.points.len() > 60 {
                spark.points[spark.points.len() - 60..].to_vec()
            } else {
                spark.points.clone()
            };

            let bar: String = points
                .iter()
                .map(|&v| {
                    let idx = (v.clamp(0.0, 1.0) * 8.0).round() as usize;
                    blocks[idx.min(8)]
                })
                .collect();

            lines.push((
                format!("  {} {} {:.1}", spark.label, bar, spark.current),
                spark.color,
            ));
        }

        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_bar_empty() {
        let renderer = MyakuRenderer::new(Theme::default());
        let bar = renderer.text_bar(0.0, 100.0, 10);
        assert_eq!(bar, "[----------]");
    }

    #[test]
    fn text_bar_full() {
        let renderer = MyakuRenderer::new(Theme::default());
        let bar = renderer.text_bar(100.0, 100.0, 10);
        assert_eq!(bar, "[##########]");
    }

    #[test]
    fn text_bar_half() {
        let renderer = MyakuRenderer::new(Theme::default());
        let bar = renderer.text_bar(50.0, 100.0, 10);
        assert_eq!(bar, "[#####-----]");
    }

    #[test]
    fn percent_color_green() {
        let renderer = MyakuRenderer::new(Theme::default());
        assert_eq!(renderer.percent_color(30.0), renderer.theme.success);
    }

    #[test]
    fn percent_color_yellow() {
        let renderer = MyakuRenderer::new(Theme::default());
        assert_eq!(renderer.percent_color(60.0), renderer.theme.warning);
    }

    #[test]
    fn percent_color_red() {
        let renderer = MyakuRenderer::new(Theme::default());
        assert_eq!(renderer.percent_color(90.0), renderer.theme.error);
    }

    #[test]
    fn sparkline_text_rendering() {
        let renderer = MyakuRenderer::new(Theme::default());
        let lines = renderer.render_sparklines_as_text();
        // Default state has no sparklines
        assert!(lines.is_empty());
    }

    #[test]
    fn glyph_color_conversion() {
        let color = to_glyph_color(&[1.0, 0.5, 0.0, 1.0]);
        // Just verify it doesn't panic — glyphon Color is opaque
        let _ = color;
    }
}
