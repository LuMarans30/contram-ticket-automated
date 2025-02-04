use std::{
    fs::File,
    io::{Error, ErrorKind, Seek, SeekFrom, Write},
    path::PathBuf,
};

use std::fs::OpenOptions;

use crate::User;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct TelegramUser {
    pub username: String,
    pub user_data: User,
}

pub struct FileManager {
    file: File,
    pub users: Vec<TelegramUser>,
}

impl FileManager {
    pub fn new(path: &str) -> Self {
        let path_buf = PathBuf::from(path);

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(false)
            .create(true)
            .open(&path_buf)
            .expect("Failed to open or create file");

        let users: Vec<TelegramUser> = match serde_json::from_reader(&file) {
            Ok(users) => users,
            Err(e) => {
                if e.is_eof() {
                    Vec::new()
                } else {
                    panic!("Failed to parse users: {}", e);
                }
            }
        };

        Self { file, users }
    }

    pub fn delete_user(&mut self, username: String) -> Result<(), Error> {
        if self.get_user(username.clone()).is_err() {
            return Err(Error::new(ErrorKind::NotFound, "User not found"));
        }

        self.users.retain(|user| user.username != username);
        self.update_json_file()
    }

    pub fn get_user(&self, username: String) -> Result<TelegramUser, Error> {
        match self
            .users
            .iter()
            .find(|user| user.username == username)
            .cloned()
        {
            Some(user) => Ok(user),
            None => Err(Error::new(ErrorKind::NotFound, "User not found")),
        }
    }

    pub fn update_json_file(&mut self) -> Result<(), Error> {
        self.file.seek(SeekFrom::Start(0))?;
        self.file.set_len(0)?;

        let serialized = serde_json::to_string_pretty(&self.users)?;
        self.file.write_all(serialized.as_bytes())?;
        self.file.flush()?;
        Ok(())
    }

    pub fn add_user(&mut self, user: TelegramUser) -> Result<(), Error> {
        self.users.push(user);
        self.update_json_file()
    }
}
