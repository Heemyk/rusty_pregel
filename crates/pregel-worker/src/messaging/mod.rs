//! Message inbox, outbox, and routing.

pub mod inbox;
pub mod outbox;
pub mod router;

pub use inbox::MessageInbox;
pub use outbox::MessageOutbox;
pub use router::MessageRouter;
