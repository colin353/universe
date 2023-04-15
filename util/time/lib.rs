pub fn timestamp_usec() -> u64 {
    let now = std::time::SystemTime::now();
    let since_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap();
    (since_epoch.as_secs() as u64) * 1_000_000 + (since_epoch.subsec_nanos() / 1000) as u64
}

pub fn timestamp() -> u64 {
    let now = std::time::SystemTime::now();
    let since_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap();
    since_epoch.as_secs()
}

pub fn fmt_timestamp(ts: u64) -> String {
    fmt_timestamp_usec(ts * 1_000_000)
}

pub fn fmt_timestamp_usec(ts: u64) -> String {
    let now = timestamp_usec();

    let suffix = if now > ts { " ago" } else { " from now" };

    let seconds = if now > ts {
        (now - ts) / 1_000_000
    } else {
        (ts - now) / 1_000_000
    };

    if seconds < 10 {
        "just now".to_string()
    } else {
        format!(
            "{}{suffix}",
            fmt_duration(std::time::Duration::from_secs(seconds))
        )
    }
}

pub fn fmt_duration(duration: std::time::Duration) -> String {
    let seconds = duration.as_secs();

    if seconds == 1 {
        return format!("1 second");
    } else if seconds < 60 {
        return format!("{} seconds", seconds);
    } else if seconds < 120 {
        return format!("1 minute");
    }

    let minutes = seconds / 60;
    if minutes == 1 {
        return format!("1 minute");
    } else if minutes < 60 {
        return format!("{} minutes", minutes);
    } else if minutes < 120 {
        return format!("1 hour");
    }

    let hours = minutes / 60;
    if hours < 24 {
        return format!("{} hours", hours);
    } else if hours < 48 {
        return format!("1 day");
    }

    let days = hours / 24;
    if days == 1 {
        return format!("1 day");
    } else if days < 7 {
        return format!("{} days", days);
    } else if days < 8 {
        return format!("1 week");
    }

    return format!("{} days", days);
}
