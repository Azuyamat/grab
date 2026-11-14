use std::fmt::Display;

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{DefaultTerminal, style::{Color, Style}, widgets::{Block, List, ListDirection, ListState}};

enum Action {
    Continue,
    Enter,
    Cancel
}

pub struct MultiSelect<T: Display> {
    items: Vec<T>,
    selected_indices: Vec<bool>,
    state: ListState,
}

impl<T: Display> MultiSelect<T> {
    pub fn new(items: Vec<T>) -> Self {
        let selected_indices = vec![false; items.len()];
        Self {
            items,
            selected_indices,
            state: ListState::default().with_selected(Some(0)),
        }
    }
    
    pub fn run(mut self, terminal: &mut DefaultTerminal) -> std::io::Result<Option<Vec<T>>> {
        loop {
            self.render(terminal)?;
            let event = crossterm::event::read()?;
            if let Event::Key(key_event) = event {
                match self.handle_input(key_event) {
                    Action::Continue => {}
                    Action::Cancel => return Ok(None),
                    Action::Enter => return Ok(Some(self.into_selected())),
                }
            }
        }
    }

    fn render(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        terminal.draw(|frame| {
            let items = self.items.iter().enumerate().map(|(i, item)| {
                let checkbox = if self.selected_indices[i] { "[x]" } else { "[ ]" };
                format!("{checkbox} {item}")
            });
            let list = List::new(items)
                .block(Block::bordered().title("Grab items"))
                .highlight_style(Style::default().fg(Color::Yellow))
                .direction(ListDirection::TopToBottom);

            frame.render_stateful_widget(list, frame.area(), &mut self.state);
        })?;
        Ok(())
    }

    fn handle_input(&mut self, input: crossterm::event::KeyEvent) -> Action {
        if input.kind != KeyEventKind::Press {
            return Action::Continue;
        }

        match (input.code, input.modifiers) {
            (KeyCode::Up, _) | (KeyCode::Char('k'), _) => {
               self.state.select_previous();
            }
            (KeyCode::Down, _) | (KeyCode::Char('j'), _) => {
               self.state.select_next();
            }
            (KeyCode::Char(' '), _) => {
                if let Some(selected) = self
                    .state
                    .selected()
                    .and_then(|index| self.selected_indices.get_mut(index))
                {
                    *selected = !*selected;
                }
            }
            (KeyCode::Char('q'), _) | (KeyCode::Esc, _) | (KeyCode::Char('c'), KeyModifiers::CONTROL)=> {
                return Action::Cancel;
            }
            (KeyCode::Enter, _) => {
                return Action::Enter;
            }
            (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                self.selected_indices = vec![true; self.items.len()];
            }
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                self.selected_indices = vec![false; self.items.len()];
            }
            (KeyCode::Char('g'), _) => {
                self.state.select_first();
            }
            (KeyCode::Char('G'), _) => {
                self.state.select_last();
            }
            _ => {}
        }
        Action::Continue
    }

    fn into_selected(self) -> Vec<T> {
        self.items
            .into_iter()
            .zip(self.selected_indices)
            .filter_map(|(item, selected)| selected.then_some(item))
            .collect()
    }
}

