#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum FriendRole {
    Guest = 1,
    Moderator = 2,
    Host = 3,
    Admin = 4,
    Bot = 5,
}

impl TryFrom<i32> for FriendRole {
    type Error = ();

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(FriendRole::Guest),
            2 => Ok(FriendRole::Moderator),
            3 => Ok(FriendRole::Host),
            4 => Ok(FriendRole::Admin),
            5 => Ok(FriendRole::Bot),
            _ => Err(()),
        }
    }
}

impl From<FriendRole> for i32 {
    fn from(role: FriendRole) -> Self {
        role as i32
    }
}

impl FriendRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            FriendRole::Guest => "guest",
            FriendRole::Moderator => "moderator",
            FriendRole::Host => "host",
            FriendRole::Admin => "admin",
            FriendRole::Bot => "bot",
        }
    }
}