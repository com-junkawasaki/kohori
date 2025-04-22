use thiserror::Error;

/// Errors that can occur in Kohori
#[derive(Error, Debug)]
pub enum KohoriError {
    #[error("Schema error: {0}")]
    Schema(String),
    
    #[error("Migration error: {0}")]
    Migration(String),
    
    #[error("RLS policy error: {0}")]
    RLSPolicy(String),
    
    #[error("SQL generation error: {0}")]
    SQLGeneration(String),
    
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Invalid type: {0}")]
    InvalidType(String),
    
    #[error("Other error: {0}")]
    Other(String),
} 