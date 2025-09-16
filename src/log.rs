#[derive(Debug, Clone)]
pub struct LogLine {
    pub content: String,
}

#[derive(Debug, Default)]
pub struct LogBuffer {
    pub lines: Vec<LogLine>,
    pub current_line: usize,
}
