use crate::{filter::FilterPattern, highlighter::HighlightPattern, processing::apply_filters};
use rayon::prelude::*;
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::mpsc,
    time::{MissedTickBehavior, interval},
};

#[derive(Debug, Clone)]
pub struct ProcessedLine {
    pub line_content: String,
    pub passes_filter: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ProcessingContext {
    pub filter_patterns: Vec<FilterPattern>,
    pub search_pattern: Option<String>,
    pub search_case_sensitive: bool,
    pub event_patterns: Vec<HighlightPattern>,
}

pub struct LogProcessor {
    input_rx: mpsc::UnboundedReceiver<String>,
    output_tx: mpsc::UnboundedSender<Vec<ProcessedLine>>,
    context_rx: mpsc::UnboundedReceiver<ProcessingContext>,
    current_context: ProcessingContext,
}

impl LogProcessor {
    pub fn new(
        input_rx: mpsc::UnboundedReceiver<String>,
        output_tx: mpsc::UnboundedSender<Vec<ProcessedLine>>,
        context_rx: mpsc::UnboundedReceiver<ProcessingContext>,
    ) -> Self {
        Self {
            input_rx,
            output_tx,
            context_rx,
            current_context: ProcessingContext::default(),
        }
    }

    pub async fn run(mut self) {
        const BATCH_SIZE: usize = 5;
        const BATCH_TIMEOUT_MS: u64 = 100;

        let mut batched_lines = Vec::with_capacity(BATCH_SIZE);
        let mut interval = interval(Duration::from_millis(BATCH_TIMEOUT_MS));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                biased; // causes select to poll the futures in the order they appear from top to bottom (https://docs.rs/tokio/latest/tokio/macro.select.html#fairness)

                _ = self.output_tx.closed() => {
                    break;
                }

                Some(new_context) = self.context_rx.recv() => {
                    self.current_context = new_context;
                }

                _ = interval.tick() => {
                    if !batched_lines.is_empty()
                        && let Some(processed) = self.process(&mut batched_lines)
                            && self.output_tx.send(processed).is_err() {
                                break;
                            }
                }

                result = self.input_rx.recv() => {
                    match result {
                        Some(line) => {
                            batched_lines.push(line);

                            if batched_lines.len() >= BATCH_SIZE
                                && let Some(processed) = self.process(&mut batched_lines)
                                    && self.output_tx.send(processed).is_err() {
                                        break;
                                    }
                        }
                        None => { // processor is being shut down, process remaining lines
                            if !batched_lines.is_empty()
                                && let Some(processed) = self.process(&mut batched_lines) {
                                    let _ = self.output_tx.send(processed);
                                }
                            break;
                        }
                    }
                }
            }
        }
    }

    fn process(&self, batch: &mut Vec<String>) -> Option<Vec<ProcessedLine>> {
        if batch.is_empty() {
            return None;
        }

        let context = &self.current_context;
        let filter_patterns = Arc::new(context.filter_patterns.clone());

        let processed: Vec<ProcessedLine> = batch
            .par_drain(..)
            .map(|line_content| {
                let passes_filter = apply_filters(&line_content, &filter_patterns);

                ProcessedLine {
                    line_content,
                    passes_filter,
                }
            })
            .collect();

        Some(processed)
    }
}

#[derive(Debug)]
pub struct ProcessorHandle {
    pub input_tx: mpsc::UnboundedSender<String>,
    pub context_tx: mpsc::UnboundedSender<ProcessingContext>,
}

impl ProcessorHandle {
    pub fn spawn(output_tx: mpsc::UnboundedSender<Vec<ProcessedLine>>) -> Self {
        let (input_tx, input_rx) = mpsc::unbounded_channel();
        let (context_tx, context_rx) = mpsc::unbounded_channel();

        let processor = LogProcessor::new(input_rx, output_tx, context_rx);

        tokio::spawn(async move {
            processor.run().await;
        });

        Self {
            input_tx,
            context_tx,
        }
    }

    pub fn update_context(&self, context: ProcessingContext) {
        let _ = self.context_tx.send(context);
    }

    pub fn send_line(&self, line: String) {
        let _ = self.input_tx.send(line);
    }
}
