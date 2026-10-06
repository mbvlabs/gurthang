use crate::user::CreateUserData;

pub struct UserFactory {
    email: String,
    password: String,
}

impl Default for UserFactory {
    fn default() -> Self {
        Self {
            email: "user@example.com".into(),
            password: "correct horse battery staple".into(),
        }
    }
}

impl UserFactory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn email(mut self, email: impl Into<String>) -> Self {
        self.email = email.into();
        self
    }

    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = password.into();
        self
    }

    pub fn data(self) -> CreateUserData {
        CreateUserData {
            email: self.email,
            password: self.password,
        }
    }
}
