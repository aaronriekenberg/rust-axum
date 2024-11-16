use std::time::SystemTime;

pub fn current_timestamp_string() -> String {
    jiff::Zoned::now().to_string()
}

pub fn system_time_to_string(system_time: SystemTime) -> String {
    match jiff::Zoned::try_from(system_time) {
        Err(_) => "UNKNOWN".to_string(),
        Ok(zoned) => zoned.to_string(),
    }
}
