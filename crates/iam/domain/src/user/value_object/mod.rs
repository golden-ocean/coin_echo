mod data_scope;
mod email;
mod employment_status;
mod gender;
mod password_credential;
mod phone;
mod staff_no;

pub use data_scope::{DataScope, DataScopeError};
pub use email::{Email, EmailError};
pub use employment_status::{EmploymentStatus, EmploymentStatusError};
pub use gender::{Gender, GenderError};
pub use password_credential::{PasswordCredential, PasswordCredentialError};
pub use phone::{Phone, PhoneError};
pub use staff_no::{StaffNo, StaffNoError};
