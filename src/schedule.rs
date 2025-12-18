use chrono::{DateTime, Local, Timelike};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct DailyWindow {
    start_minutes: u16,
    end_minutes: u16,
    label: String,
}

#[derive(Debug, Clone)]
pub enum ScheduleStatus {
    Active { remaining: Duration },
    Inactive { starts_in: Duration },
}

impl DailyWindow {
    pub fn parse(spec: &str) -> Result<Self, String> {
        let (start_raw, end_raw) = split_window(spec)
            .ok_or_else(|| "expected <start>-<end> like \"9am-5pm\"".to_string())?;

        let start_minutes = parse_time_of_day(&start_raw)?;
        let end_minutes = parse_time_of_day(&end_raw)?;

        let label = format!(
            "{:02}:{:02}-{:02}:{:02}",
            start_minutes / 60,
            start_minutes % 60,
            end_minutes / 60,
            end_minutes % 60
        );

        Ok(Self {
            start_minutes: start_minutes as u16,
            end_minutes: end_minutes as u16,
            label,
        })
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn status(&self, now: DateTime<Local>) -> ScheduleStatus {
        const SECONDS_PER_DAY: u32 = 24 * 60 * 60;

        let start = u32::from(self.start_minutes) * 60;
        let end = u32::from(self.end_minutes) * 60;
        let now_seconds = now.num_seconds_from_midnight();

        if start == end {
            return ScheduleStatus::Active {
                remaining: Duration::from_secs(SECONDS_PER_DAY as u64),
            };
        }

        if start < end {
            if now_seconds >= start && now_seconds < end {
                let remaining = end - now_seconds;
                return ScheduleStatus::Active {
                    remaining: Duration::from_secs(remaining as u64),
                };
            }

            let until_start = if now_seconds < start {
                start - now_seconds
            } else {
                (SECONDS_PER_DAY - now_seconds) + start
            };

            return ScheduleStatus::Inactive {
                starts_in: Duration::from_secs(until_start as u64),
            };
        }

        // Wrap-around window (e.g., 22:00-06:00).
        if now_seconds >= start || now_seconds < end {
            let target = if now_seconds >= start {
                (SECONDS_PER_DAY + end) - now_seconds
            } else {
                end - now_seconds
            };

            return ScheduleStatus::Active {
                remaining: Duration::from_secs(target as u64),
            };
        }

        let until_start = start - now_seconds;
        ScheduleStatus::Inactive {
            starts_in: Duration::from_secs(until_start as u64),
        }
    }

    pub fn start_minutes(&self) -> u16 {
        self.start_minutes
    }
}

fn split_window(spec: &str) -> Option<(String, String)> {
    for delimiter in ['-', '\u{2013}', '\u{2014}'] {
        if let Some(index) = spec.find(delimiter) {
            let (start, end) = spec.split_at(index);
            let end = &end[delimiter.len_utf8()..];
            return Some((start.trim().to_string(), end.trim().to_string()));
        }
    }

    let lower = spec.to_lowercase();
    if let Some(index) = lower.find(" to ") {
        let start = &spec[..index];
        let end = &spec[index + 4..];
        return Some((start.trim().to_string(), end.trim().to_string()));
    }

    None
}

fn parse_time_of_day(raw: &str) -> Result<u32, String> {
    let trimmed = raw.trim().to_lowercase();
    let (time_part, suffix) = if let Some(stripped) = trimmed.strip_suffix("am") {
        (stripped.trim(), Some("am"))
    } else if let Some(stripped) = trimmed.strip_suffix("pm") {
        (stripped.trim(), Some("pm"))
    } else {
        (trimmed.as_str(), None)
    };

    let mut parts = time_part.split(':');
    let hour_str = parts
        .next()
        .ok_or_else(|| "missing hour in time".to_string())?;
    let minute_str = parts.next();

    if parts.next().is_some() {
        return Err("invalid time format".to_string());
    }

    let mut hour: u32 = hour_str
        .parse()
        .map_err(|_| format!("invalid hour: {hour_str}"))?;

    let minute: u32 = match minute_str {
        Some(val) if !val.is_empty() => {
            val.parse().map_err(|_| format!("invalid minutes: {val}"))?
        }
        _ => 0,
    };

    if minute >= 60 {
        return Err("minutes must be between 0 and 59".to_string());
    }

    match suffix {
        Some("am") => {
            if hour == 12 {
                hour = 0;
            } else if hour == 0 || hour > 12 {
                return Err("hour must be between 1 and 12 when using am/pm".to_string());
            }
        }
        Some("pm") => {
            if hour == 12 {
                hour = 12;
            } else if hour == 0 || hour > 12 {
                return Err("hour must be between 1 and 12 when using am/pm".to_string());
            } else {
                hour += 12;
            }
        }
        _ => {
            if hour > 23 {
                return Err("hour must be between 0 and 23".to_string());
            }
        }
    }

    Ok(hour * 60 + minute)
}
