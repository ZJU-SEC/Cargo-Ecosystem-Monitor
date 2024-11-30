#[derive(Debug)]
pub struct CondRuf {
    pub cond: Option<String>,
    pub feature: String,
}

#[derive(Debug)]
pub struct CondRufs(Vec<CondRuf>);

#[derive(Debug)]
pub enum RufStatus {
    Unknown,
    Active,
    Incomplete,
    Accepted,
    Removed,
}

impl From<&str> for RufStatus {
    fn from(value: &str) -> Self {
        match value {
            "active" => RufStatus::Active,
            "incomplete" => RufStatus::Incomplete,
            "accepted" => RufStatus::Accepted,
            "removed" => RufStatus::Removed,
            "" => RufStatus::Unknown,
            _ => unreachable!("Fatal, unknown ruf status: {}", value),
        }
    }
}

impl From<u32> for RufStatus {
    fn from(value: u32) -> Self {
        match value {
            0 => RufStatus::Unknown,
            1 => RufStatus::Active,
            2 => RufStatus::Incomplete,
            3 => RufStatus::Accepted,
            4 => RufStatus::Removed,
            _ => unreachable!("Fatal, unknown ruf status: {}", value),
        }
    }
}
