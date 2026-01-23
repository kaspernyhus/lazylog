# Timeline View Implementation Plan

## Overview
Add a new "Timeline View" that visualizes event density across time as a heatmap. Each row represents an event type, each column a time slot, with block characters (░▒▓█) showing intensity.

## User Requirements
- Access via `Shift+T` from LogView
- View-only for MVP (Escape/q to exit)
- Block characters for intensity visualization
- Ignore log lines without parseable timestamps

## Files to Modify

### 1. `src/timeline.rs` (NEW)
Create new module with:
```rust
pub struct TimeSlot {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub event_counts: HashMap<String, usize>,
}

pub struct TimelineData {
    pub slots: Vec<TimeSlot>,
    pub event_names: Vec<String>,
    pub max_count: usize,
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
}

impl TimelineData {
    pub fn compute(log_buffer: &LogBuffer, event_tracker: &LogEventTracker, slot_count: usize) -> Option<Self>
    pub fn intensity_char(count: usize, max_count: usize) -> char
}
```

**Algorithm for `compute()`:**
1. Collect all log lines with valid timestamps
2. Build `line_index -> timestamp` lookup map
3. Find min/max timestamps to get time range
4. Divide time range into `slot_count` slots
5. For each event, lookup its line's timestamp, increment slot count
6. Track max_count for intensity normalization

**Intensity mapping:**
- 0: ` ` (space)
- 1-25%: `░`
- 25-50%: `▒`
- 50-75%: `▓`
- 75-100%: `█`

### 2. `src/lib.rs`
Add: `pub mod timeline;`

### 3. `src/app.rs`
- Add `TimelineView` to `ViewState` enum
- Add field: `timeline_data: Option<TimelineData>`
- Add method:
```rust
pub fn activate_timeline_view(&mut self) {
    let slot_count = self.viewport.width.saturating_sub(20) as usize; // Reserve space for labels
    self.timeline_data = TimelineData::compute(&self.log_buffer, &self.event_tracker, slot_count);
    self.set_view_state(ViewState::TimelineView);
}
```

### 4. `src/command.rs`
- Add `ActivateTimelineView` to `Command` enum
- Add description: `"View event timeline"`
- Add execute handler: `app.activate_timeline_view()`

### 5. `src/keybindings.rs`
- In `register_log_view_bindings()`: add `Shift+T` -> `ActivateTimelineView`
- Add new `register_timeline_view_bindings()`:
  - `q` -> `Quit`
  - `Esc` -> `Cancel` (back to LogView)
- Call new method from `new()` and register global bindings

### 6. `src/ui/colors.rs`
Add:
```rust
// Timeline
pub const TIMELINE_BORDER: Color = Color::Magenta;
pub const TIMELINE_LABEL_FG: Color = Color::Yellow;
pub const TIMELINE_EMPTY_FG: Color = Color::DarkGray;
```

### 7. `src/ui/mod.rs`
Add rendering dispatch in main `render()` match:
```rust
ViewState::TimelineView => {
    let timeline_area = popup_area(area, 100, 30);
    self.render_timeline(timeline_area, buf);
}
```

### 8. `src/ui/lists.rs`
Add `render_timeline()` method:
- Clear area, draw bordered block with title " Event Timeline "
- Handle None case: "No timestamp data available"
- Handle empty events: "No events found"
- Layout: [event labels | heatmap cells]
- Render event names as Y-axis labels (truncate if needed)
- Render heatmap: for each row/event, for each col/slot, draw intensity char
- Show time range at bottom: `HH:MM:SS - HH:MM:SS`

### 9. `src/help.rs`
Add Timeline View section in `build_from_registry()` after Events View section.

## Edge Cases
- No timestamps in log -> show "No timestamp data available"
- No events found -> show "No events found"
- Single timestamp (no time span) -> show message
- Many event types -> truncate to viewport height
- Event on line without timestamp -> skip that event

## Verification
1. `cargo check` - compiles without errors
2. `cargo test` - all tests pass
3. Manual test: load a log file with timestamps and events, press `Shift+T`, verify:
   - Timeline view appears
   - Event names shown on left
   - Heatmap shows intensity based on event density
   - Time range displayed
   - `q` or `Esc` returns to LogView
4. Test edge case: log without timestamps shows appropriate message
