use crate::log::LogLine;
use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt::Debug;

/// Tags that can be attached to visible lines for rendering metadata
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tag {
    /// Line is selected
    Selected,
    /// Line passes normal text filters
    Filtered,
    /// Line is marked
    Marked,
    /// Line contains an event
    Event,
    /// Line belongs to an enabled file
    FileEnabled,
}

/// Trait for rules that determine line visibility.
pub trait VisibilityRule {
    /// Returns true if the line should be visible
    fn is_visible(&self, line: &LogLine) -> bool;
}

/// Trait for rules that add tags to lines.
pub trait TagRule {
    /// Returns tags to attach to this line, or None
    fn get_tags(&self, line: &LogLine) -> Option<Tag>;
}

/// A visible line with metadata for rendering
#[derive(Debug, Clone)]
pub struct VisibleLine {
    /// Index in the original log.
    pub log_index: usize,
    /// A set of tags applied to the line.
    pub tags: HashSet<Tag>,
}

impl VisibleLine {
    fn new(log_index: usize) -> Self {
        Self {
            log_index,
            tags: HashSet::new(),
        }
    }

    pub fn add_tag(&mut self, tag: Tag) {
        self.tags.insert(tag);
    }

    pub fn remove_tag(&mut self, tag: Tag) {
        self.tags.remove(&tag);
    }
}

/// The main viewport resolver that applies rules to determine visible lines
pub struct ViewportResolver {
    /// Visibility rules (filters, file enables) - determine which lines show
    visibility_rules: Vec<Box<dyn VisibilityRule>>,
    /// Tag rules (marks, events) - add metadata to visible lines
    tag_rules: Vec<Box<dyn TagRule>>,
    /// Cached visible lines
    visible_cache: RefCell<Option<Vec<VisibleLine>>>,
    /// Cache generation counter for invalidation
    cache_generation: u64,
}

impl Debug for ViewportResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ViewportResolver")
            .field("visibility_rules_count", &self.visibility_rules.len())
            .field("tag_rules_count", &self.tag_rules.len())
            .field("cache_generation", &self.cache_generation)
            .field("has_cache", &self.visible_cache.borrow().is_some())
            .finish()
    }
}

impl Default for ViewportResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewportResolver {
    /// Create a new empty resolver
    pub fn new() -> Self {
        Self {
            visibility_rules: Vec::new(),
            tag_rules: Vec::new(),
            visible_cache: RefCell::new(None),
            cache_generation: 0,
        }
    }

    /// Add a visibility rule (filters, file enables)
    pub fn add_visibility_rule(&mut self, rule: Box<dyn VisibilityRule>) {
        self.visibility_rules.push(rule);
        self.invalidate_cache();
    }

    /// Add a tag rule (marks, events)
    pub fn add_tag_rule(&mut self, rule: Box<dyn TagRule>) {
        self.tag_rules.push(rule);
        self.invalidate_cache();
    }

    /// Clear all rules
    pub fn clear_rules(&mut self) {
        self.visibility_rules.clear();
        self.tag_rules.clear();
        self.invalidate_cache();
    }

    /// Invalidate the cache, forcing recomputation on next access
    pub fn invalidate_cache(&mut self) {
        *self.visible_cache.borrow_mut() = None;
        self.cache_generation += 1;
    }

    /// Get the visible lines (cached or compute)
    /// Returns a clone of the cached visible lines to avoid borrowing issues
    pub fn get_visible_lines(&self, lines: &[LogLine]) -> Vec<VisibleLine> {
        // Check cache first
        let cache = self.visible_cache.borrow();
        if let Some(cached) = cache.as_ref() {
            return cached.clone();
        }
        drop(cache);

        // Compute and cache
        let visible = self.compute_visible_lines(lines);
        *self.visible_cache.borrow_mut() = Some(visible.clone());
        visible
    }

    /// Get an iterator over visible log line references
    pub fn get_visible_lines_iter<'a>(&self, lines: &'a [LogLine]) -> impl Iterator<Item = &'a LogLine> {
        let visible_lines = self.get_visible_lines(lines);
        visible_lines.into_iter().map(move |vl| &lines[vl.log_index])
    }

    /// Compute visible lines by applying all rules
    fn compute_visible_lines(&self, lines: &[LogLine]) -> Vec<VisibleLine> {
        let mut results = Vec::new();

        for (idx, line) in lines.iter().enumerate() {
            // Step 1: Check visibility rules - ALL must return true
            let is_visible = if self.visibility_rules.is_empty() {
                // No visibility rules means all lines visible
                true
            } else {
                self.visibility_rules.iter().all(|rule| rule.is_visible(line))
            };

            if !is_visible {
                continue;
            }

            // Step 2: Line is visible, collect tags from tag rules
            let mut visible_line = VisibleLine::new(idx);
            for tag_rule in &self.tag_rules {
                if let Some(tag) = tag_rule.get_tags(line) {
                    visible_line.add_tag(tag);
                }
            }

            results.push(visible_line);
        }

        results
    }

    /// Convert viewport index to log index
    pub fn viewport_to_log(&mut self, viewport_idx: usize, lines: &[LogLine]) -> Option<usize> {
        let visible = self.get_visible_lines(lines);
        visible.get(viewport_idx).map(|v| v.log_index)
    }

    /// Convert log index to viewport index
    pub fn log_to_viewport(&mut self, log_idx: usize, lines: &[LogLine]) -> Option<usize> {
        let visible = self.get_visible_lines(lines);
        visible.iter().position(|v| v.log_index == log_idx)
    }

    /// Get the total count of visible lines
    pub fn visible_count(&self, lines: &[LogLine]) -> usize {
        self.get_visible_lines(lines).len()
    }

    /// Update Tag::Marked on cached visible lines without full recomputation.
    pub fn update_mark_tags(&mut self, marked_indices: &HashSet<usize>) {
        let mut cache = self.visible_cache.borrow_mut();
        if let Some(visible_lines) = cache.as_mut() {
            for visible_line in visible_lines.iter_mut() {
                let line_index = visible_line.log_index;
                if marked_indices.contains(&line_index) {
                    visible_line.add_tag(Tag::Marked);
                } else {
                    visible_line.remove_tag(Tag::Marked);
                }
            }
        }
    }
}
