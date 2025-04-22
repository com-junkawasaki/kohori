use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::Write;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use crate::{Schema, Result, KohoriError};
use crate::dialect::Dialect;

/// A database migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Migration {
    pub id: String,
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub schema: Schema,
    pub operations: Vec<MigrationOperation>,
    pub checksum: Option<String>,
    pub applied: bool,
}

impl Migration {
    /// Create a new migration for the given schema
    pub fn new(name: String, schema: Schema) -> Self {
        let timestamp = Utc::now();
        let id = format!("{}_{}", timestamp.format("%Y%m%d%H%M%S"), name);
        
        Self {
            id,
            name,
            timestamp,
            schema,
            operations: Vec::new(),
            checksum: None,
            applied: false,
        }
    }

    /// Add a migration operation
    pub fn add_operation(&mut self, operation: MigrationOperation) -> &mut Self {
        self.operations.push(operation);
        self
    }

    /// Generate SQL for this migration using the provided dialect
    pub fn to_sql(&self, dialect: &dyn Dialect) -> String {
        let mut sql = String::new();
        
        // Add migration header comment
        sql.push_str(&format!("-- Migration: {}\n", self.id));
        sql.push_str(&format!("-- Created at: {}\n\n", self.timestamp));
        
        // Begin transaction
        sql.push_str("BEGIN;\n\n");
        
        // Add operations
        for operation in &self.operations {
            sql.push_str(&operation.to_sql(dialect));
            sql.push_str("\n\n");
        }
        
        // Record migration
        sql.push_str(&format!("INSERT INTO _kohori_migrations (id, name, applied_at) VALUES ('{}', '{}', CURRENT_TIMESTAMP);\n\n", 
            self.id, self.name));
        
        // Commit transaction
        sql.push_str("COMMIT;\n");
        
        sql
    }

    /// Compute a checksum for this migration
    pub fn compute_checksum(&mut self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use serde_json::json;
        
        let data = json!({
            "name": self.name,
            "timestamp": self.timestamp.to_rfc3339(),
            "operations": self.operations,
        });
        
        let serialized = serde_json::to_string(&data).unwrap_or_default();
        
        let mut hasher = DefaultHasher::new();
        serialized.hash(&mut hasher);
        let hash = hasher.finish();
        
        let checksum = format!("{:016x}", hash);
        self.checksum = Some(checksum.clone());
        
        checksum
    }

    /// Write this migration to a file
    pub fn write_to_file(&self, dir: impl AsRef<Path>) -> Result<PathBuf> {
        let dir = dir.as_ref();
        if !dir.exists() {
            fs::create_dir_all(dir)?;
        }
        
        let filename = format!("{}_{}.sql", self.timestamp.format("%Y%m%d%H%M%S"), self.name);
        let path = dir.join(filename);
        
        let mut file = File::create(&path)?;
        file.write_all(self.to_sql(&crate::dialect::PostgreSQL::default()).as_bytes())?;
        
        Ok(path)
    }
}

/// A database migration operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationOperation {
    /// Create a new table
    CreateTable {
        table: crate::schema::Table,
    },
    /// Alter an existing table
    AlterTable {
        table: String,
        changes: Vec<TableChange>,
    },
    /// Drop a table
    DropTable {
        table: String,
        cascade: bool,
    },
    /// Create an RLS policy
    CreateRLSPolicy {
        table: String,
        policy: crate::rls::Policy,
    },
    /// Drop an RLS policy
    DropRLSPolicy {
        table: String,
        policy: String,
    },
    /// Enable RLS on a table
    EnableRLS {
        table: String,
        force: bool,
    },
    /// Disable RLS on a table
    DisableRLS {
        table: String,
    },
    /// Execute raw SQL
    RawSQL {
        sql: String,
        description: Option<String>,
    },
}

impl MigrationOperation {
    /// Generate SQL for this operation using the provided dialect
    pub fn to_sql(&self, dialect: &dyn Dialect) -> String {
        match self {
            MigrationOperation::CreateTable { table } => {
                dialect.create_table(table)
            },
            MigrationOperation::AlterTable { table, changes } => {
                dialect.alter_table(table, changes)
            },
            MigrationOperation::DropTable { table, cascade } => {
                dialect.drop_table(table, *cascade)
            },
            MigrationOperation::CreateRLSPolicy { table, policy } => {
                dialect.create_rls_policy(table, policy)
            },
            MigrationOperation::DropRLSPolicy { table, policy } => {
                dialect.drop_rls_policy(table, policy)
            },
            MigrationOperation::EnableRLS { table, force } => {
                dialect.enable_rls(table, *force)
            },
            MigrationOperation::DisableRLS { table } => {
                dialect.disable_rls(table)
            },
            MigrationOperation::RawSQL { sql, description } => {
                let mut result = String::new();
                if let Some(desc) = description {
                    result.push_str(&format!("-- {}\n", desc));
                }
                result.push_str(sql);
                result
            }
        }
    }
}

/// A change to a table structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TableChange {
    /// Add a column
    AddColumn(crate::schema::Column),
    /// Drop a column
    DropColumn {
        name: String,
        cascade: bool,
    },
    /// Alter a column
    AlterColumn {
        name: String,
        change: ColumnChange,
    },
    /// Add a constraint
    AddConstraint(crate::schema::Constraint),
    /// Drop a constraint
    DropConstraint {
        name: String,
        cascade: bool,
    },
    /// Rename table
    RenameTable {
        new_name: String,
    },
    /// Rename column
    RenameColumn {
        old_name: String,
        new_name: String,
    },
}

/// A change to a column
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColumnChange {
    /// Change data type
    SetDataType(crate::schema::DataType),
    /// Set NOT NULL constraint
    SetNotNull,
    /// Drop NOT NULL constraint
    DropNotNull,
    /// Set default value
    SetDefault(String),
    /// Drop default value
    DropDefault,
}

/// A migration manager
#[derive(Debug)]
pub struct MigrationManager {
    migrations_dir: PathBuf,
    applied_migrations: HashMap<String, DateTime<Utc>>,
    dialect: Box<dyn Dialect>,
}

impl MigrationManager {
    /// Create a new migration manager
    pub fn new(migrations_dir: impl Into<PathBuf>, dialect: Box<dyn Dialect>) -> Self {
        Self {
            migrations_dir: migrations_dir.into(),
            applied_migrations: HashMap::new(),
            dialect,
        }
    }

    /// Create a new migration
    pub fn create_migration(&self, name: impl Into<String>, schema: Schema) -> Migration {
        Migration::new(name.into(), schema)
    }

    /// Save a migration to disk
    pub fn save_migration(&self, migration: &Migration) -> Result<PathBuf> {
        migration.write_to_file(&self.migrations_dir)
    }

    /// Generate SQL for a migration
    pub fn generate_sql(&self, migration: &Migration) -> String {
        migration.to_sql(&*self.dialect)
    }

    /// Load all migrations from disk
    pub fn load_migrations(&mut self) -> Result<Vec<Migration>> {
        let entries = fs::read_dir(&self.migrations_dir)
            .map_err(|e| KohoriError::IO(e))?;
        
        let migrations = Vec::new();
        
        for entry in entries {
            let entry = entry.map_err(|e| KohoriError::IO(e))?;
            let path = entry.path();
            
            if path.is_file() && path.extension().map_or(false, |ext| ext == "sql") {
                // TODO: Parse migration file
                // For now, we'll just log that we found it
                println!("Found migration file: {}", path.display());
            }
        }
        
        Ok(migrations)
    }

    /// Diff two schemas and generate migration operations
    pub fn diff_schemas(&self, from: &Schema, to: &Schema) -> Vec<MigrationOperation> {
        let mut operations = Vec::new();
        
        // Track existing tables in the "from" schema
        let mut from_tables: HashMap<String, &crate::schema::Table> = HashMap::new();
        
        // TODO: Implement schema diffing logic
        // This is a complex algorithm to detect:
        // - New tables
        // - Dropped tables
        // - Changed tables (new/dropped/altered columns, constraints, etc.)
        // - RLS changes
        
        operations
    }
}

/// A migration plan
#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationPlan {
    pub name: String,
    pub operations: Vec<MigrationOperation>,
}

impl MigrationPlan {
    /// Create a new migration plan
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            operations: Vec::new(),
        }
    }

    /// Add an operation to the plan
    pub fn add_operation(&mut self, operation: MigrationOperation) -> &mut Self {
        self.operations.push(operation);
        self
    }

    /// Convert this plan to a migration
    pub fn to_migration(&self, schema: Schema) -> Migration {
        let mut migration = Migration::new(self.name.clone(), schema);
        migration.operations = self.operations.clone();
        migration.compute_checksum();
        migration
    }
} 