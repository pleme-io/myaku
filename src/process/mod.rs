//! Process list management: sorting, filtering, display formatting.

use crate::metrics::memory::format_bytes;
use crate::platform::ProcessInfo;

/// Sort column for the process list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    Cpu,
    Memory,
    Pid,
    Name,
}

impl SortColumn {
    /// Cycle to the next sort column.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Cpu => Self::Memory,
            Self::Memory => Self::Pid,
            Self::Pid => Self::Name,
            Self::Name => Self::Cpu,
        }
    }

    /// Column header label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Cpu => "CPU%",
            Self::Memory => "MEM",
            Self::Pid => "PID",
            Self::Name => "NAME",
        }
    }

    /// Parse from config string.
    #[must_use]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "memory" | "mem" => Self::Memory,
            "pid" => Self::Pid,
            "name" => Self::Name,
            _ => Self::Cpu,
        }
    }
}

/// Managed process list with sorting, filtering, and selection.
pub struct ProcessList {
    /// All processes (unfiltered, unsorted).
    all_processes: Vec<ProcessInfo>,
    /// Filtered + sorted view indices into `all_processes`.
    view: Vec<usize>,
    /// Current sort column.
    pub sort_column: SortColumn,
    /// Sort ascending (true) or descending (false).
    pub sort_ascending: bool,
    /// Current filter string (empty = no filter).
    pub filter: String,
    /// Currently selected index in the view.
    pub selected: usize,
    /// Scroll offset for the visible window.
    pub scroll_offset: usize,
    /// Number of visible rows.
    pub visible_rows: usize,
}

#[allow(dead_code)]
impl ProcessList {
    /// Create a new empty process list.
    #[must_use]
    pub fn new(sort_column: SortColumn, visible_rows: usize) -> Self {
        Self {
            all_processes: Vec::new(),
            view: Vec::new(),
            sort_column,
            sort_ascending: false,
            filter: String::new(),
            selected: 0,
            scroll_offset: 0,
            visible_rows,
        }
    }

    /// Update with new process data.
    pub fn update(&mut self, processes: Vec<ProcessInfo>) {
        self.all_processes = processes;
        self.rebuild_view();
    }

    /// Rebuild the filtered + sorted view.
    fn rebuild_view(&mut self) {
        // Filter
        let filter_lower = self.filter.to_lowercase();
        self.view = self
            .all_processes
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                if filter_lower.is_empty() {
                    return true;
                }
                p.name.to_lowercase().contains(&filter_lower)
                    || p.pid.to_string().contains(&filter_lower)
            })
            .map(|(i, _)| i)
            .collect();

        // Sort
        let procs = &self.all_processes;
        let col = self.sort_column;
        let asc = self.sort_ascending;
        self.view.sort_by(|&a, &b| {
            let ord = match col {
                SortColumn::Cpu => procs[a]
                    .cpu
                    .partial_cmp(&procs[b].cpu)
                    .unwrap_or(std::cmp::Ordering::Equal),
                SortColumn::Memory => procs[a].memory.cmp(&procs[b].memory),
                SortColumn::Pid => procs[a].pid.cmp(&procs[b].pid),
                SortColumn::Name => procs[a]
                    .name
                    .to_lowercase()
                    .cmp(&procs[b].name.to_lowercase()),
            };
            if asc { ord } else { ord.reverse() }
        });

        // Clamp selection
        if !self.view.is_empty() {
            self.selected = self.selected.min(self.view.len() - 1);
        } else {
            self.selected = 0;
        }
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        if !self.view.is_empty() && self.selected + 1 < self.view.len() {
            self.selected += 1;
            self.ensure_visible();
        }
    }

    /// Move selection up.
    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.ensure_visible();
        }
    }

    /// Page down.
    pub fn page_down(&mut self) {
        if self.view.is_empty() {
            return;
        }
        self.selected = (self.selected + self.visible_rows).min(self.view.len() - 1);
        self.ensure_visible();
    }

    /// Page up.
    pub fn page_up(&mut self) {
        self.selected = self.selected.saturating_sub(self.visible_rows);
        self.ensure_visible();
    }

    /// Jump to first.
    pub fn select_first(&mut self) {
        self.selected = 0;
        self.ensure_visible();
    }

    /// Jump to last.
    pub fn select_last(&mut self) {
        if !self.view.is_empty() {
            self.selected = self.view.len() - 1;
        }
        self.ensure_visible();
    }

    /// Cycle sort column.
    pub fn cycle_sort(&mut self) {
        self.sort_column = self.sort_column.next();
        self.rebuild_view();
    }

    /// Toggle sort direction.
    pub fn toggle_sort_direction(&mut self) {
        self.sort_ascending = !self.sort_ascending;
        self.rebuild_view();
    }

    /// Set filter string and rebuild view.
    pub fn set_filter(&mut self, filter: String) {
        self.filter = filter;
        self.selected = 0;
        self.scroll_offset = 0;
        self.rebuild_view();
    }

    /// Get the currently selected process, if any.
    #[must_use]
    pub fn selected_process(&self) -> Option<&ProcessInfo> {
        self.view
            .get(self.selected)
            .map(|&idx| &self.all_processes[idx])
    }

    /// Get the visible slice of processes for rendering.
    #[must_use]
    pub fn visible_processes(&self) -> Vec<&ProcessInfo> {
        let end = (self.scroll_offset + self.visible_rows).min(self.view.len());
        self.view[self.scroll_offset..end]
            .iter()
            .map(|&idx| &self.all_processes[idx])
            .collect()
    }

    /// Total number of processes in the filtered view.
    #[must_use]
    pub fn filtered_count(&self) -> usize {
        self.view.len()
    }

    /// Total number of processes (unfiltered).
    #[must_use]
    pub fn total_count(&self) -> usize {
        self.all_processes.len()
    }

    /// Format a process row for display.
    #[must_use]
    pub fn format_row(proc: &ProcessInfo) -> String {
        format!(
            "{:>7} {:<20} {:>6.1} {:>10} {:<6} {}",
            proc.pid,
            truncate_name(&proc.name, 20),
            proc.cpu,
            format_bytes(proc.memory),
            truncate_name(&proc.status, 6),
            truncate_name(&proc.user, 10),
        )
    }

    /// Header row for the process table.
    #[must_use]
    pub fn header_row(&self) -> String {
        let sort_indicator = |col: SortColumn| -> &str {
            if col == self.sort_column {
                if self.sort_ascending { " ^" } else { " v" }
            } else {
                ""
            }
        };
        format!(
            "{:>7} {:<20} {:>6}{} {:>10}{} {:<6} {}",
            format!("PID{}", sort_indicator(SortColumn::Pid)),
            format!("NAME{}", sort_indicator(SortColumn::Name)),
            "CPU%",
            sort_indicator(SortColumn::Cpu),
            "MEM",
            sort_indicator(SortColumn::Memory),
            "STATE",
            "USER",
        )
    }

    fn ensure_visible(&mut self) {
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + self.visible_rows {
            self.scroll_offset = self.selected + 1 - self.visible_rows;
        }
    }
}

/// Truncate a string to max_len, adding ".." if truncated.
fn truncate_name(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len > 2 {
        format!("{}..", &s[..max_len - 2])
    } else {
        s[..max_len].to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_processes() -> Vec<ProcessInfo> {
        vec![
            ProcessInfo {
                pid: 100,
                name: "alpha".into(),
                cpu: 50.0,
                memory: 1024 * 1024 * 100,
                status: "Run".into(),
                parent_pid: 1,
                user: "root".into(),
            },
            ProcessInfo {
                pid: 200,
                name: "bravo".into(),
                cpu: 10.0,
                memory: 1024 * 1024 * 500,
                status: "Sleep".into(),
                parent_pid: 1,
                user: "drzzln".into(),
            },
            ProcessInfo {
                pid: 300,
                name: "charlie".into(),
                cpu: 80.0,
                memory: 1024 * 1024 * 200,
                status: "Run".into(),
                parent_pid: 1,
                user: "drzzln".into(),
            },
        ]
    }

    #[test]
    fn sort_by_cpu_desc() {
        let mut pl = ProcessList::new(SortColumn::Cpu, 20);
        pl.update(sample_processes());
        let visible = pl.visible_processes();
        assert_eq!(visible[0].pid, 300); // 80% cpu
        assert_eq!(visible[1].pid, 100); // 50% cpu
        assert_eq!(visible[2].pid, 200); // 10% cpu
    }

    #[test]
    fn sort_by_name_asc() {
        let mut pl = ProcessList::new(SortColumn::Name, 20);
        pl.sort_ascending = true;
        pl.update(sample_processes());
        let visible = pl.visible_processes();
        assert_eq!(visible[0].name, "alpha");
        assert_eq!(visible[1].name, "bravo");
        assert_eq!(visible[2].name, "charlie");
    }

    #[test]
    fn filter_by_name() {
        let mut pl = ProcessList::new(SortColumn::Cpu, 20);
        pl.update(sample_processes());
        pl.set_filter("bravo".into());
        assert_eq!(pl.filtered_count(), 1);
        assert_eq!(pl.visible_processes()[0].name, "bravo");
    }

    #[test]
    fn filter_by_pid() {
        let mut pl = ProcessList::new(SortColumn::Cpu, 20);
        pl.update(sample_processes());
        pl.set_filter("200".into());
        assert_eq!(pl.filtered_count(), 1);
    }

    #[test]
    fn cycle_sort() {
        let mut pl = ProcessList::new(SortColumn::Cpu, 20);
        pl.cycle_sort();
        assert_eq!(pl.sort_column, SortColumn::Memory);
        pl.cycle_sort();
        assert_eq!(pl.sort_column, SortColumn::Pid);
    }

    #[test]
    fn selection_navigation() {
        let mut pl = ProcessList::new(SortColumn::Cpu, 20);
        pl.update(sample_processes());
        assert_eq!(pl.selected, 0);
        pl.select_next();
        assert_eq!(pl.selected, 1);
        pl.select_prev();
        assert_eq!(pl.selected, 0);
        pl.select_last();
        assert_eq!(pl.selected, 2);
        pl.select_first();
        assert_eq!(pl.selected, 0);
    }

    #[test]
    fn truncate() {
        assert_eq!(truncate_name("hello", 10), "hello");
        assert_eq!(truncate_name("hello world!", 7), "hello..");
        assert_eq!(truncate_name("ab", 2), "ab");
    }
}
