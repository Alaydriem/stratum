use amm_abi::types::AmmLogLevel;
use curia::{Fields, Level, LogEvent, Logger};

// One shim log line, sent up through the mod's manager registration.
pub struct StLogLine;

impl StLogLine {
    const TARGET: &'static str = "alaydriem_stratum::shim";

    // `level` is an AmmLogLevel value and `fields_json` a JSON object; a line logged before the mod is
    // registered is dropped, because Curia has no sink yet.
    pub fn emit(level: u8, message: &str, fields_json: &str, file: &str, line: u32) {
        let Some(level) = Self::level(level) else { return };
        let fields = match serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(fields_json) {
            Ok(map) => Fields::from_map(map),
            Err(_) => Fields::new(),
        };
        Logger::emit(LogEvent {
            level,
            target: Self::TARGET.to_string(),
            message: message.to_string(),
            fields,
            timestamp: chrono::Utc::now(),
            file: Some(file.to_string()),
            line: Some(line),
        });
    }

    fn level(code: u8) -> Option<Level> {
        match code {
            code if code == AmmLogLevel::Error as u8 => Some(Level::Error),
            code if code == AmmLogLevel::Warn as u8 => Some(Level::Warn),
            code if code == AmmLogLevel::Info as u8 => Some(Level::Info),
            code if code == AmmLogLevel::Debug as u8 => Some(Level::Debug),
            code if code == AmmLogLevel::Trace as u8 => Some(Level::Trace),
            _ => None,
        }
    }
}
