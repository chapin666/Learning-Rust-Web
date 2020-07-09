
use crate::api_error::ApiError;
use crate::db;
use crate::schema::user;
use chrono::{ NaiveDateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use argon2::Config;
use rand::Rng;

#[derive(Serialize, Deserialize, AsChangeset)]
#[table_name = "user"]
pub struct UserMessage {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, Serialize, Queryable, Insertable)]
#[table_name = "user"]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub password: String,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

impl User {

    pub fn find_by_email(email: String) -> Result<Self, ApiError> {
        let conn = db::connection()?;

        println!("{}", email);

        let user = user::table
            .filter(user::email.eq(email))
            .first(&conn)?;

        Ok(user)
    }

    pub fn find_all() -> Result<Vec<Self>, ApiError> {
        let conn = db::connection()?;

        let users = user::table.load::<User>(&conn)?;

        Ok(users)
    }

    pub fn find(id: Uuid) -> Result<Self, ApiError> {
        let conn = db::connection()?;

        let user = user::table
            .filter(user::id.eq(id))
            .first(&conn)?;

        Ok(user)
    }

    pub fn create(user: UserMessage) -> Result<Self, ApiError> {
        let conn = db::connection()?;

        let mut user = User::from(user);
        user.hash_passsword()?;

        let user = diesel::insert_into(user::table)
            .values(user)
            .get_result(&conn)?;

        Ok(user)
    }

    pub fn update(id: Uuid, user: UserMessage) -> Result<Self, ApiError> {
        let conn = db::connection()?;

        let user = diesel::update(user::table)
            .filter(user::id.eq(id))
            .set(user)
            .get_result(&conn)?;

        Ok(user)
    }

    pub fn delete(id: Uuid) -> Result<usize, ApiError> {
        let conn = db::connection()?;

        let res = diesel::delete(
                user::table
                    .filter(user::id.eq(id))
            )
            .execute(&conn)?;

        Ok(res)
    }

    pub fn hash_passsword(&mut self) -> Result<(), ApiError> {
        let salt: [u8; 32] = rand::thread_rng().gen();
        let config = Config::default();

        self.password = argon2::hash_encoded(self.password.as_bytes(), &salt, &config)
            .map_err(|e| ApiError::new(500, format!("Failed to hash password: {}", e)))?;

        Ok(())
    }

    pub fn verify_password(&self, password: &[u8]) -> Result<bool, ApiError> {
        argon2::verify_encoded(&self.password, password)
            .map_err(|e| ApiError::new(500, format!("Failed to verify password: {}", e)))
    }

}


impl From<UserMessage> for User {
    fn from(user: UserMessage) -> Self {
        User {
            id: Uuid::new_v4(),
            email: user.email,
            password: user.password,
            created_at: Utc::now().naive_utc(),
            updated_at: None,
        }
    }
}