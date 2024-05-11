use crate::models::users::User;
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct UsersListResponse {
    pub users: Vec<User>,
    pub results: usize,
}
