#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    #[serde(rename = "EmailAcquirente")]
    personal_email: String,
    #[serde(rename = "Nominativi[0].Nome")]
    first_name: String,
    #[serde(rename = "Nominativi[0].Cognome")]
    last_name: String,
    #[serde(rename = "Nominativi[0].Email")]
    email: String,
    #[serde(rename = "Nominativi[0].Telefono")]
    phone: String,
}

impl User {
    pub fn new(
        personal_email: String,
        first_name: String,
        last_name: String,
        email: String,
        phone: String,
    ) -> Self {
        Self {
            personal_email,
            first_name,
            last_name,
            email,
            phone,
        }
    }

    pub fn get_email(&self) -> String {
        self.email.clone()
    }

    pub fn get_phone(&self) -> String {
        self.phone.clone()
    }

    pub fn get_personal_email(&self) -> String {
        self.personal_email.clone()
    }

    pub fn get_last_name(&self) -> String {
        self.last_name.clone()
    }

    pub fn get_first_name(&self) -> String {
        self.first_name.clone()
    }
}

impl Display for User {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "First Name: {}\nLast Name: {}\nEmail: {}\nPersonal Email: {}\nPhone: {}",
            self.first_name, self.last_name, self.email, self.personal_email, self.phone
        )
    }
}
