use color_eyre::eyre::OptionExt;
use futures::{FutureExt, StreamExt};
use ratatui::crossterm::event::Event as CrosstermEvent;
use std::io::{BufRead, BufReader};
use std::time::Duration;
use tokio::sync::mpsc;

/// The frequency at which tick events are emitted.
const TICK_FPS: f64 = 30.0;

/// Representation of all possible events.
#[derive(Clone, Debug)]
pub enum Event {
    /// An event that is emitted on a regular schedule.
    ///
    /// Use this event to run any code which has to run outside of being a direct response to a user
    /// event. e.g. polling exernal systems, updating animations, or rendering the UI based on a
    /// fixed frame rate.
    Tick,
    /// Crossterm events.
    ///
    /// These events are emitted by the terminal.
    Crossterm(CrosstermEvent),
    /// Application events.
    ///
    /// Use this event to emit custom events that are specific to your application.
    App(AppEvent),
}

/// Application events.
///
/// You can extend this enum with your own custom events.
#[derive(Clone, Debug)]
pub enum AppEvent {
    /// Quit the application.
    Quit,
    /// Confirm
    Confirm,
    /// Cancel
    Cancel,
    /// Move up
    MoveUp,
    /// Move down
    MoveDown,
    /// Page up
    PageUp,
    /// Page down
    PageDown,
    /// Center viewport on selected line
    CenterSelected,
    /// Goto top
    GotoTop,
    /// Goto bottom
    GotoBottom,
    /// Goto specific line input mode
    ActivateGotoLineMode,
    /// Scroll left horizontally
    ScrollLeft,
    /// Scroll right horizontally
    ScrollRight,
    /// Reset horizontal scroll
    ResetHorizontal,
    /// Toggle help popup
    ToggleHelp,
    /// Start search input mode
    ActivateSearchMode,
    /// Toggle case sensitivity
    ToggleCaseSensitive,
    /// Go to next search match
    SearchNext,
    /// Go to previous search match
    SearchPrevious,
    /// Start filter input mode
    ActivateFilterMode,
    /// Toggle filter (include/exclude)
    ToggleFilterMode,
    /// Activate filter list view
    ActivateFilterListView,
    /// Toggle selected filter pattern active/inactive
    ToggleFilterPatternActive,
    /// Remove selected filter pattern
    RemoveFilterPattern,
    /// Navigate to previous search history
    SearchHistoryPrevious,
    /// Navigate to next search history
    SearchHistoryNext,
    /// New line received from stdin
    NewLine(String),
    /// Toggle follow mode
    ToggleFollowMode,
    /// Activate options view
    ActivateOptionsView,
    /// Toggle selected display option
    ToggleDisplayOption,
}

/// Terminal event handler.
#[derive(Debug)]
pub struct EventHandler {
    /// Event sender channel.
    sender: mpsc::UnboundedSender<Event>,
    /// Event receiver channel.
    receiver: mpsc::UnboundedReceiver<Event>,
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler {
    /// Constructs a new instance of [`EventHandler`] and spawns a new thread to handle events.
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let actor = EventTask::new(sender.clone());
        tokio::spawn(async { actor.run().await });
        Self { sender, receiver }
    }

    /// Constructs a new instance with stdin reader task enabled.
    pub fn new_with_stdin() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let actor = EventTask::new(sender.clone());
        tokio::spawn(async { actor.run().await });

        let stdin_sender = sender.clone();
        std::thread::spawn(move || {
            stdin_reader_task(stdin_sender);
        });

        Self { sender, receiver }
    }

    /// Receives an event from the sender.
    ///
    /// This function blocks until an event is received.
    ///
    /// # Errors
    ///
    /// This function returns an error if the sender channel is disconnected. This can happen if an
    /// error occurs in the event thread. In practice, this should not happen unless there is a
    /// problem with the underlying terminal.
    pub async fn next(&mut self) -> color_eyre::Result<Event> {
        self.receiver
            .recv()
            .await
            .ok_or_eyre("Failed to receive event")
    }

    /// Queue an app event to be sent to the event receiver.
    ///
    /// This is useful for sending events to the event handler which will be processed by the next
    /// iteration of the application's event loop.
    pub fn send(&mut self, app_event: AppEvent) {
        // Ignore the result as the reciever cannot be dropped while this struct still has a
        // reference to it
        let _ = self.sender.send(Event::App(app_event));
    }
}

/// A thread that handles reading crossterm events and emitting tick events on a regular schedule.
struct EventTask {
    /// Event sender channel.
    sender: mpsc::UnboundedSender<Event>,
}

impl EventTask {
    /// Constructs a new instance of [`EventThread`].
    fn new(sender: mpsc::UnboundedSender<Event>) -> Self {
        Self { sender }
    }

    /// Runs the event thread.
    ///
    /// This function emits tick events at a fixed rate and polls for crossterm events in between.
    async fn run(self) -> color_eyre::Result<()> {
        let tick_rate = Duration::from_secs_f64(1.0 / TICK_FPS);
        let mut reader = crossterm::event::EventStream::new();
        let mut tick = tokio::time::interval(tick_rate);
        loop {
            let tick_delay = tick.tick();
            let crossterm_event = reader.next().fuse();
            tokio::select! {
              _ = self.sender.closed() => {
                break;
              }
              _ = tick_delay => {
                self.send(Event::Tick);
              }
              Some(Ok(evt)) = crossterm_event => {
                self.send(Event::Crossterm(evt));
              }
            };
        }
        Ok(())
    }

    /// Sends an event to the receiver.
    fn send(&self, event: Event) {
        // Ignores the result because shutting down the app drops the receiver, which causes the send
        // operation to fail. This is expected behavior and should not panic.
        let _ = self.sender.send(event);
    }
}

/// Background task that reads lines from stdin and sends them as events.
fn stdin_reader_task(sender: mpsc::UnboundedSender<Event>) {
    use std::sync::mpsc as std_mpsc;
    use std::time::Duration;

    let stdin = std::io::stdin();
    let reader = BufReader::new(stdin);

    let (line_sender, line_receiver) = std_mpsc::channel::<String>();

    // Spawn a thread to read from stdin
    std::thread::spawn(move || {
        for line in reader.lines() {
            match line {
                Ok(content) => {
                    if line_sender.send(content).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Batch lines and send them periodically
    const BATCH_INTERVAL: Duration = Duration::from_millis(50);
    const MAX_BATCH_SIZE: usize = 100;

    let mut batch = Vec::new();
    loop {
        // Try to receive lines for the batch interval
        let deadline = std::time::Instant::now() + BATCH_INTERVAL;

        loop {
            let timeout = deadline.saturating_duration_since(std::time::Instant::now());
            if timeout.is_zero() {
                break;
            }

            match line_receiver.recv_timeout(timeout) {
                Ok(line) => {
                    batch.push(line);
                    if batch.len() >= MAX_BATCH_SIZE {
                        break;
                    }
                }
                Err(std_mpsc::RecvTimeoutError::Timeout) => break,
                Err(std_mpsc::RecvTimeoutError::Disconnected) => {
                    // Send remaining batch and exit
                    for line in batch.drain(..) {
                        let _ = sender.send(Event::App(AppEvent::NewLine(line)));
                    }
                    return;
                }
            }
        }

        // Send batch
        for line in batch.drain(..) {
            if sender.send(Event::App(AppEvent::NewLine(line))).is_err() {
                return;
            }
        }
    }
}
