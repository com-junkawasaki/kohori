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
        let mut from_tables = HashMap::new();
        for table in &from.tables {
            from_tables.insert(table.name.clone(), table);
        }
        
        // Track existing tables in the "to" schema
        let mut to_tables = HashMap::new();
        for table in &to.tables {
            to_tables.insert(table.name.clone(), table);
        }
        
        // Find tables that were added
        for table in &to.tables {
            if !from_tables.contains_key(&table.name) {
                // New table
                operations.push(MigrationOperation::CreateTable {
                    table: table.clone(),
                });
            }
        }
        
        // Find tables that were removed
        for table in &from.tables {
            if !to_tables.contains_key(&table.name) {
                // Dropped table
                operations.push(MigrationOperation::DropTable {
                    table: table.name.clone(),
                    cascade: false,
                });
            }
        }
        
        // Find tables that were changed
        for (table_name, to_table) in &to_tables {
            if let Some(from_table) = from_tables.get(table_name) {
                let mut changes = Vec::new();
                
                // Check for columns that were added
                let from_columns: HashMap<String, &crate::schema::Column> = from_table.columns.iter()
                    .map(|c| (c.name.clone(), c))
                    .collect();
                    
                for to_column in &to_table.columns {
                    if !from_columns.contains_key(&to_column.name) {
                        // New column
                        changes.push(TableChange::AddColumn(to_column.clone()));
                    } else {
                        // Column exists, check for changes
                        let from_column = from_columns[&to_column.name];
                        
                        // Check data type
                        if from_column.data_type != to_column.data_type {
                            changes.push(TableChange::AlterColumn {
                                name: to_column.name.clone(),
                                change: ColumnChange::SetDataType(to_column.data_type.clone()),
                            });
                        }
                        
                        // Check nullable
                        if from_column.is_nullable && !to_column.is_nullable {
                            changes.push(TableChange::AlterColumn {
                                name: to_column.name.clone(),
                                change: ColumnChange::SetNotNull,
                            });
                        } else if !from_column.is_nullable && to_column.is_nullable {
                            changes.push(TableChange::AlterColumn {
                                name: to_column.name.clone(),
                                change: ColumnChange::DropNotNull,
                            });
                        }
                        
                        // Check default
                        match (&from_column.default_value, &to_column.default_value) {
                            (None, Some(value)) => {
                                changes.push(TableChange::AlterColumn {
                                    name: to_column.name.clone(),
                                    change: ColumnChange::SetDefault(value.clone()),
                                });
                            },
                            (Some(_), None) => {
                                changes.push(TableChange::AlterColumn {
                                    name: to_column.name.clone(),
                                    change: ColumnChange::DropDefault,
                                });
                            },
                            (Some(from_value), Some(to_value)) if from_value != to_value => {
                                changes.push(TableChange::AlterColumn {
                                    name: to_column.name.clone(),
                                    change: ColumnChange::SetDefault(to_value.clone()),
                                });
                            },
                            _ => {}
                        }
                        
                        // More column changes could be checked here
                    }
                }
                
                // Check for columns that were dropped
                let to_columns: HashMap<String, &crate::schema::Column> = to_table.columns.iter()
                    .map(|c| (c.name.clone(), c))
                    .collect();
                    
                for from_column in &from_table.columns {
                    if !to_columns.contains_key(&from_column.name) {
                        // Dropped column
                        changes.push(TableChange::DropColumn {
                            name: from_column.name.clone(),
                            cascade: false,
                        });
                    }
                }
                
                // Check for RLS policy changes
                if !from_table.rls_enabled && to_table.rls_enabled {
                    // RLS was enabled
                    operations.push(MigrationOperation::EnableRLS {
                        table: table_name.clone(),
                        force: false,
                    });
                } else if from_table.rls_enabled && !to_table.rls_enabled {
                    // RLS was disabled
                    operations.push(MigrationOperation::DisableRLS {
                        table: table_name.clone(),
                    });
                }
                
                // Find policies that were added
                let from_policies: HashMap<String, &crate::rls::Policy> = from_table.rls_policies.iter()
                    .map(|p| (p.name.clone(), p))
                    .collect();
                    
                for to_policy in &to_table.rls_policies {
                    if !from_policies.contains_key(&to_policy.name) {
                        // New policy
                        operations.push(MigrationOperation::CreateRLSPolicy {
                            table: table_name.clone(),
                            policy: to_policy.clone(),
                        });
                    }
                    // Policy changes could be checked here
                }
                
                // Find policies that were dropped
                let to_policies: HashMap<String, &crate::rls::Policy> = to_table.rls_policies.iter()
                    .map(|p| (p.name.clone(), p))
                    .collect();
                    
                for from_policy in &from_table.rls_policies {
                    if !to_policies.contains_key(&from_policy.name) {
                        // Dropped policy
                        operations.push(MigrationOperation::DropRLSPolicy {
                            table: table_name.clone(),
                            policy: from_policy.name.clone(),
                        });
                    }
                }
                
                // If there are any changes to the table, add an AlterTable operation
                if !changes.is_empty() {
                    operations.push(MigrationOperation::AlterTable {
                        table: table_name.clone(),
                        changes,
                    });
                }
            }
        }
        
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