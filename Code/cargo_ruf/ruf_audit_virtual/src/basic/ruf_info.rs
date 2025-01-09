#[derive(Debug, Clone)]
pub struct CondRuf {
    pub cond: Option<String>,
    pub feature: String,
}

#[derive(Debug, Clone)]
pub struct CondRufs(Vec<CondRuf>);

#[derive(Debug, Clone)]
pub enum RufStatus {
    Unknown,
    Active,
    Incomplete,
    Accepted,
    Removed,
}

impl CondRufs {
    pub fn new(rufs: Vec<CondRuf>) -> Self {
        CondRufs(rufs)
    }

    pub fn empty() -> Self {
        CondRufs(Vec::new())
    }

    pub fn push(&mut self, ruf: CondRuf) {
        self.0.push(ruf);
    }

    pub fn extend(&mut self, rufs: impl IntoIterator<Item = CondRuf>) {
        self.0.extend(rufs.into_iter());
    }

    pub fn borrow(&self) -> Vec<&CondRuf> {
        self.0.iter().collect()
    }

    pub fn inner(self) -> Vec<CondRuf> {
        self.0.into_iter().collect()
    }
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

impl RufStatus {
    pub fn is_usable(&self) -> bool {
        match self {
            Self::Removed | Self::Unknown => false,
            _ => true,
        }
    }
}
