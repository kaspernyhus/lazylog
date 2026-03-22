use crate::{
    app::{App, Overlay, ViewState},
    ui::colors::{EXPLORER_BORDER, EXPLORER_DIR_FG, EXPLORER_HIGHLIGHT_DIR_FG, EXPLORER_HIGHLIGHT_ITEM_FG},
};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, Clear, Widget, WidgetRef},
};
use ratatui_explorer::{FileExplorerBuilder, Input as ExplorerInput, Theme as ExplorerTheme};

fn build_theme() -> ExplorerTheme {
    ExplorerTheme::default()
        .with_block(
            Block::default()
                .title(" Add File ")
                .title_alignment(Alignment::Center)
                .title_style(Style::default().bold())
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(EXPLORER_BORDER)),
        )
        .with_dir_style(Style::default().fg(EXPLORER_DIR_FG))
        .with_highlight_dir_style(
            Style::default()
                .fg(EXPLORER_HIGHLIGHT_DIR_FG)
                .add_modifier(Modifier::BOLD),
        )
        .with_highlight_item_style(
            Style::default()
                .fg(EXPLORER_HIGHLIGHT_ITEM_FG)
                .add_modifier(Modifier::BOLD),
        )
        .with_highlight_symbol("▶ ")
}

impl App {
    pub fn activate_add_file_overlay(&mut self) {
        if self.view_state == ViewState::FilesView
            && let Ok(explorer) = FileExplorerBuilder::build_with_theme(build_theme())
        {
            self.file_explorer = Some(explorer);
            self.show_overlay(Overlay::AddFile);
        }
    }

    pub fn handle_file_explorer_event(&mut self, key: KeyEvent) {
        let input = match key.code {
            KeyCode::Esc => {
                self.close_overlay();
                return;
            }
            KeyCode::Enter => {
                if let Some(explorer) = &self.file_explorer {
                    let current = explorer.current();
                    if current.is_file() {
                        let path = current.path.to_string_lossy().into_owned();
                        self.close_overlay();
                        self.add_file(path);
                        return;
                    }
                }
                ExplorerInput::Right
            }
            KeyCode::Up | KeyCode::Char('k') => ExplorerInput::Up,
            KeyCode::Down | KeyCode::Char('j') => ExplorerInput::Down,
            KeyCode::Left | KeyCode::Char('h') => ExplorerInput::Left,
            KeyCode::Right | KeyCode::Char('l') => ExplorerInput::Right,
            KeyCode::Home => ExplorerInput::Home,
            KeyCode::End => ExplorerInput::End,
            KeyCode::PageUp => ExplorerInput::PageUp,
            KeyCode::PageDown => ExplorerInput::PageDown,
            _ => return,
        };
        if let Some(explorer) = &mut self.file_explorer {
            let _ = explorer.handle(input);
        }
    }

    pub(super) fn render_file_explorer(&self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        if let Some(explorer) = &self.file_explorer {
            explorer.widget().render_ref(area, buf);
        }
    }
}
