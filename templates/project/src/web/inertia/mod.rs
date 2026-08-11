mod page;
mod request;
mod response;
mod ssr;

pub use request::InertiaRequest;
pub use response::{InertiaRenderer, external_location, mutation_redirect};
pub use ssr::InertiaSsr;

// Deliberately omitted in this proof of concept: deferred/optional/merge/once
// props, infinite scroll metadata, Precognition, and history encryption.
