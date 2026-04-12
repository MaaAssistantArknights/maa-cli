pub type CowStr = std::borrow::Cow<'static, str>;

mod confirm;
mod inquiry;
mod select;

pub use confirm::Confirm;
pub use inquiry::Inquiry;
pub use select::{Select, SelectD, Selectable, ValueWithDesc};
