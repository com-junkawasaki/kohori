pub mod schema;
pub mod migration;
pub mod rls;
pub mod dialect;
pub mod error;

pub use error::KohoriError;
pub type Result<T> = std::result::Result<T, KohoriError>;

use serde::{Serialize, Deserialize};

/// Represents a database schema definition that can be used to generate migrations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub name: String,
    pub tables: Vec<schema::Table>,
}

impl Schema {
    /// Create a new schema with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tables: Vec::new(),
        }
    }

    /// Add a table to the schema
    pub fn add_table(&mut self, table: schema::Table) -> &mut Self {
        self.tables.push(table);
        self
    }

    /// Generate a migration from the current schema state
    pub fn generate_migration(&self, name: impl Into<String>) -> migration::Migration {
        migration::Migration::new(name.into(), self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Column, DataType, Table};
    use crate::rls::{Policy, PolicyTarget, SecurityContext};

    #[test]
    fn test_simple_schema() {
        let mut schema = Schema::new("public");
        
        let users = Table::new("users")
            .column(Column::new("id", DataType::UUID).primary_key().default("gen_random_uuid()"))
            .column(Column::new("email", DataType::Text).not_null())
            .rls_policy(
                Policy::new("users_select_policy")
                    .using("auth.uid() = id")
                    .target(PolicyTarget::Select)
                    .security_context(SecurityContext::Authenticated)
            );
        
        schema.add_table(users.clone());
        
        let migration = schema.generate_migration("create_users");
        let sql = migration.to_sql(&dialect::PostgreSQL::default());
        
        assert!(sql.contains("CREATE TABLE"));
        assert!(sql.contains("users"));
        assert!(sql.contains("ENABLE ROW LEVEL SECURITY"));
    }
} 