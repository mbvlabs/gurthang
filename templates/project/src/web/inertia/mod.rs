mod page;
mod request;
mod response;

pub use request::InertiaRequest;
pub use response::{InertiaRenderer, external_location, mutation_redirect};

// Deliberately omitted in this proof of concept: deferred/optional/merge/once
// props, infinite scroll metadata, Precognition, history encryption, and SSR.
