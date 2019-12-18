use super::{
    authentication_barcode, authentication_password, Account, DbConnection, ServiceResult,
};

#[derive(Serialize, Deserialize)]
#[serde(tag = "method", content = "value")]
pub enum Authentication {
    #[serde(rename = "password")]
    Password { username: String, password: String },
    #[serde(rename = "barcode")]
    Barcode { code: String },
}

impl Authentication {
    pub fn get_account(&self, conn: &DbConnection) -> ServiceResult<Account> {
        match self {
            Authentication::Password { username, password } => {
                authentication_password::get(&conn, &username, &password)
            }
            Authentication::Barcode { code } => authentication_barcode::get(&conn, &code),
        }
    }
}
