#[derive(Debug, Clone)]
pub struct LogLine {
    pub index: usize,
    pub content: String,
}

#[derive(Debug, Default)]
pub struct LogBuffer {
    pub lines: Vec<LogLine>,
    pub current_index: usize,
}

impl LogBuffer {
    pub fn load_from_file(&mut self, path: &str) -> color_eyre::Result<()> {
        let content = std::fs::read_to_string(path)?;
        self.lines = content
            .lines()
            .enumerate()
            .map(|(index, line)| LogLine {
                index,
                content: line.to_string(),
            })
            .collect();
        self.current_index = 0;
        Ok(())
    }
}
