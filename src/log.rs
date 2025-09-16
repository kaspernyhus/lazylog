#[derive(Debug, Clone)]
pub struct LogLine {
    pub content: String,
}

#[derive(Debug, Default)]
pub struct LogBuffer {
    pub lines: Vec<LogLine>,
    pub current_line: usize,
}

impl LogBuffer {
    pub fn load_from_file(&mut self, path: &str) -> color_eyre::Result<()> {
        let content = std::fs::read_to_string(path)?;
        self.lines = content
            .lines()
            .map(|line| LogLine {
                content: line.to_string(),
            })
            .collect();
        self.current_line = 0;
        Ok(())
    }
}
