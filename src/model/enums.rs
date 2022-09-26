use diesel::backend::{self, Backend};
use diesel::deserialize::{self, FromSql};
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::*;

/// Represents the permission level of an account
#[derive(
    Debug, Copy, Clone, FromSqlRow, AsExpression, Hash, PartialEq, Eq, Serialize, Deserialize, Enum,
)]
#[diesel(sql_type = SmallInt)]
pub enum Permission {
    /// default user without the ability to edit anything
    Default,
    /// ascii member who can perform transactions
    Member,
    /// ascii executive or admin who can do everything
    Admin,
}

impl PartialOrd for Permission {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Permission {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.level().cmp(&other.level())
    }
}

impl Permission {
    /// Check if the permission level is `Permission::DEFAULT`
    pub fn is_default(self) -> bool {
        Permission::Default == self
    }

    /// Check if the permission level is `Permission::MEMBER`
    pub fn is_member(self) -> bool {
        Permission::Member == self
    }

    /// Check if the permission level is `Permission::ADMIN`
    pub fn is_admin(self) -> bool {
        Permission::Admin == self
    }

    pub fn level(self) -> u32 {
        match self {
            Permission::Default => 0,
            Permission::Member => 1,
            Permission::Admin => 2,
        }
    }
}

/// For manuel database convertion
impl<DB: Backend> ToSql<SmallInt, DB> for Permission
where
    i16: ToSql<SmallInt, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        match *self {
            Permission::Default => 0.to_sql(out),
            Permission::Member => 1.to_sql(out),
            Permission::Admin => 2.to_sql(out),
        }
    }
}

/// For manuel database convertion
impl<DB: Backend> FromSql<SmallInt, DB> for Permission
where
    i16: FromSql<SmallInt, DB>,
{
    fn from_sql(bytes: backend::RawValue<DB>) -> deserialize::Result<Self> {
        let v = i16::from_sql(bytes)?;
        Ok(match v {
            0 => Permission::Default,
            1 => Permission::Member,
            2 => Permission::Admin,
            _ => panic!("'{}' is not a valid permission!", &v),
        })
    }
}

/// Represents the permission level of an account
#[derive(
    Debug, Copy, Clone, FromSqlRow, AsExpression, Hash, PartialEq, Eq, Serialize, Deserialize, Enum,
)]
#[diesel(sql_type = SmallInt)]
#[serde(rename_all = "UPPERCASE")]
pub enum StampType {
    None,
    Coffee,
    Bottle,
}

impl StampType {
    pub fn is_none(self) -> bool {
        Self::None == self
    }

    pub fn is_coffee(self) -> bool {
        Self::Coffee == self
    }

    pub fn is_bottle(self) -> bool {
        Self::Bottle == self
    }
}

impl Default for StampType {
    fn default() -> Self {
        StampType::None
    }
}

/// For manuel database convertion
impl<DB: Backend> ToSql<SmallInt, DB> for StampType
where
    i16: ToSql<SmallInt, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        match *self {
            Self::None => 0.to_sql(out),
            Self::Coffee => 1.to_sql(out),
            Self::Bottle => 2.to_sql(out),
        }
    }
}

/// For manuel database convertion
impl<DB: Backend> FromSql<SmallInt, DB> for StampType
where
    i16: FromSql<SmallInt, DB>,
{
    fn from_sql(bytes: backend::RawValue<DB>) -> deserialize::Result<Self> {
        let v = i16::from_sql(bytes)?;
        Ok(match v {
            0 => Self::None,
            1 => Self::Coffee,
            2 => Self::Bottle,
            _ => panic!("'{}' is not a valid stamp type!", &v),
        })
    }
}
