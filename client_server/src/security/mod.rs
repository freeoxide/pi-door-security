//! Security utilities module

mod privileges;

pub use privileges::drop_privileges;

pub struct SecretStore;
