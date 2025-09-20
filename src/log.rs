#[derive(Debug, Clone)]
pub struct LogLine {
    pub index: usize,
    pub content: String,
}

#[derive(Debug, Default)]
pub struct LogBuffer {
    pub file_path: Option<String>,
    pub lines: Vec<LogLine>,
}

impl LogBuffer {
    pub fn load_from_file(&mut self, path: &str) -> color_eyre::Result<()> {
        let content = std::fs::read_to_string(path)?;
        self.file_path = Some(path.to_string());
        self.lines = content
            .lines()
            .enumerate()
            .map(|(index, line)| LogLine {
                index,
                content: line.to_string(),
            })
            .collect();
        Ok(())
    }

    pub fn get_lines(&self, start: usize, end: usize) -> &[LogLine] {
        &self.lines[start..end.min(self.lines.len())]
    }

    pub fn get_lines_max_length(&self, start: usize, end: usize) -> usize {
        self.lines[start..end.min(self.lines.len())]
            .iter()
            .map(|line| line.content.len())
            .max()
            .unwrap_or(0)
    }
}
