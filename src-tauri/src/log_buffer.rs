use std::fmt;
use std::sync::{Arc, Mutex};
use tracing::field::{Field, Visit};
use tracing::Subscriber;
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

const MAX_ENTRIES: usize = 200;

#[derive(Clone)]
pub struct SharedLogBuffer(Arc<Mutex<Vec<String>>>);

impl SharedLogBuffer {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(Vec::with_capacity(MAX_ENTRIES))))
    }

    pub fn get_entries(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }

    fn push(&self, entry: String) {
        let mut buf = self.0.lock().unwrap();
        if buf.len() >= MAX_ENTRIES {
            buf.remove(0);
        }
        buf.push(entry);
    }
}

pub struct LogBufferLayer {
    buffer: SharedLogBuffer,
}

impl LogBufferLayer {
    pub fn new(buffer: SharedLogBuffer) -> Self {
        Self { buffer }
    }
}

struct MessageVisitor {
    message: String,
    fields: Vec<(String, String)>,
}

impl MessageVisitor {
    fn new() -> Self {
        Self {
            message: String::new(),
            fields: Vec::new(),
        }
    }
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        } else {
            self.fields.push((field.name().to_string(), format!("{:?}", value)));
        }
    }
}

impl<S: Subscriber> Layer<S> for LogBufferLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        let level = meta.level();
        let target = meta.target();

        let mut visitor = MessageVisitor::new();
        event.record(&mut visitor);

        let now = chrono::Local::now().format("%H:%M:%S%.3f");

        let mut line = format!("{} {} [{}] {}", now, level, target, visitor.message);
        for (k, v) in &visitor.fields {
            line.push_str(&format!(" {}={}", k, v));
        }

        self.buffer.push(line);
    }
}
