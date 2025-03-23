pub mod artist;
pub mod core;
pub mod song;

pub mod shared {
    pub use crate::artist::*;
    pub use crate::core::*;
    pub use crate::song::*;
}

type Result<T> = ::core::result::Result<T, leptos::prelude::ServerFnError>;
