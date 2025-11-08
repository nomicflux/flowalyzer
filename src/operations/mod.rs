pub mod recipe;
pub mod repeat;
pub mod silence;
pub mod speed;

// Re-export operation functions for convenience
pub use repeat::repeat_chunk;
pub use silence::insert_silence;
pub use speed::change_speed;
