use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Event {
    #[serde(with = "ts_seconds")]
    timestamp: DateTime<Utc>,
    message: String,
    pub event_type: String,
    process_type: String,
    process_status: String,
    pub attributes: Option<Value>,
}

pub struct EventRecorder {
    events: Vec<Event>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum EventType {
    NewRun,
    FinishedRun,
    ToolExecution,
    FinishedToolExecution,
    ToolMetricEvent,
    MetricEvent,
    SyslogEvent,
    ErrorEvent,
    TestEvent, // Added TestEvent variant
}

impl EventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::NewRun => "new_run",
            EventType::FinishedRun => "finished_run",
            EventType::ToolExecution => "tool_execution",
            EventType::FinishedToolExecution => "finished_tool_execution",
            EventType::MetricEvent => "metric_event",
            EventType::SyslogEvent => "syslog_event",
            EventType::ToolMetricEvent => "tool_metric_event",
            EventType::ErrorEvent => "error",
            EventType::TestEvent => "test_event", // Handle TestEvent
        }
    }
}

impl EventRecorder {
    pub fn new() -> Self {
        EventRecorder { events: Vec::new() }
    }

    pub fn record_event(
        &mut self,
        event_type: EventType,
        message: String,
        attributes: Option<Value>,
        timestamp: Option<DateTime<Utc>>,
    ) {
        let event = Event {
            timestamp: timestamp.unwrap_or_else(Utc::now),
            message,
            event_type: "process_status".to_owned(),
            process_type: "pipeline".to_owned(),
            process_status: event_type.as_str().to_owned(),
            attributes,
        };
        self.events.push(event);
    }

    pub fn get_events(&self) -> &[Event] {
        &self.events
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.events.len()
    }
}

impl Default for EventRecorder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_record_event() {
        let mut recorder = EventRecorder::new();
        let message = "[event_recorder.rs]Test event".to_string();
        let attributes = Some(json!({"key": "value"}));

        recorder.record_event(
            EventType::ToolExecution,
            message.clone(),
            attributes.clone(),
            None,
        );

        assert_eq!(recorder.len(), 1);

        let event = &recorder.get_events()[0];
        assert_eq!(event.message, message);
        assert_eq!(event.event_type, "process_status");
        assert_eq!(event.process_type, "pipeline");
        assert_eq!(event.process_status, "tool_execution");
        assert_eq!(event.attributes, attributes);
    }

    #[test]
    fn test_clear_events() {
        let mut recorder = EventRecorder::new();
        recorder.record_event(
            EventType::ToolExecution,
            "Test event".to_string(),
            None,
            None,
        );
        assert_eq!(recorder.len(), 1);

        recorder.clear();
        assert!(recorder.is_empty());
    }

    #[test]
    fn test_event_type_as_str() {
        assert_eq!(EventType::FinishedRun.as_str(), "finished_run");
        assert_eq!(EventType::ToolExecution.as_str(), "tool_execution");
        assert_eq!(EventType::MetricEvent.as_str(), "metric_event");
        assert_eq!(EventType::TestEvent.as_str(), "test_event");
    }

    #[test]
    fn test_record_test_event() {
        let mut recorder = EventRecorder::new();
        let message = "Test event for testing".to_string();
        let attributes = Some(json!({"test_key": "test_value"}));

        recorder.record_event(
            EventType::TestEvent,
            message.clone(),
            attributes.clone(),
            None,
        );

        assert_eq!(recorder.len(), 1);

        let event = &recorder.get_events()[0];
        assert_eq!(event.message, message);
        assert_eq!(event.event_type, "process_status");
        assert_eq!(event.process_type, "pipeline");
        assert_eq!(event.process_status, "test_event");
        assert_eq!(event.attributes, attributes);
    }
}
