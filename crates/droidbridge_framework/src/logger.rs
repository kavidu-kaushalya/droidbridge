use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LogPriority {
    Verbose = 2,
    Debug = 3,
    Info = 4,
    Warn = 5,
    Error = 6,
    Assert = 7,
}

impl LogPriority {
    pub fn tag_letter(&self) -> &'static str {
        match self {
            LogPriority::Verbose => "V",
            LogPriority::Debug => "D",
            LogPriority::Info => "I",
            LogPriority::Warn => "W",
            LogPriority::Error => "E",
            LogPriority::Assert => "A",
        }
    }
}

static MIN_LOG_LEVEL: AtomicU8 = AtomicU8::new(LogPriority::Debug as u8);

pub struct Log;

impl Log {
    pub fn set_min_level(level: LogPriority) {
        MIN_LOG_LEVEL.store(level as u8, Ordering::Relaxed);
    }

    pub fn println(priority: LogPriority, tag: &str, msg: &str) {
        let min = MIN_LOG_LEVEL.load(Ordering::Relaxed);
        if (priority as u8) >= min {
            println!("[Logcat/{}] {}: {}", priority.tag_letter(), tag, msg);
        }
    }

    pub fn v(tag: &str, msg: &str) {
        Self::println(LogPriority::Verbose, tag, msg);
    }

    pub fn d(tag: &str, msg: &str) {
        Self::println(LogPriority::Debug, tag, msg);
    }

    pub fn i(tag: &str, msg: &str) {
        Self::println(LogPriority::Info, tag, msg);
    }

    pub fn w(tag: &str, msg: &str) {
        Self::println(LogPriority::Warn, tag, msg);
    }

    pub fn e(tag: &str, msg: &str) {
        Self::println(LogPriority::Error, tag, msg);
    }
}
