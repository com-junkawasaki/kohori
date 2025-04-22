use serde::{Serialize, Deserialize};
use crate::rls::Policy;

/// Represents a database table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub name: String,
    pub columns: Vec<Column>,
    pub constraints: Vec<Constraint>,
    pub indexes: Vec<Index>,
    pub rls_enabled: bool,
    pub rls_policies: Vec<Policy>,
    pub comment: Option<String>,
}

impl Table {
    /// Create a new table with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            columns: Vec::new(),
            constraints: Vec::new(),
            indexes: Vec::new(),
            rls_enabled: false,
            rls_policies: Vec::new(),
            comment: None,
        }
    }

    /// Add a column to the table
    pub fn column(mut self, column: Column) -> Self {
        self.columns.push(column);
        self
    }

    /// Add a constraint to the table
    pub fn constraint(mut self, constraint: Constraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Add an index to the table
    pub fn index(mut self, index: Index) -> Self {
        self.indexes.push(index);
        self
    }

    /// Enable RLS on the table
    pub fn enable_rls(mut self) -> Self {
        self.rls_enabled = true;
        self
    }

    /// Add an RLS policy to the table
    pub fn rls_policy(mut self, policy: Policy) -> Self {
        self.rls_enabled = true; // Automatically enable RLS when adding a policy
        self.rls_policies.push(policy);
        self
    }

    /// Add a comment to the table
    pub fn comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }
}

/// Represents a column in a database table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    pub data_type: DataType,
    pub is_primary_key: bool,
    pub is_nullable: bool,
    pub is_unique: bool,
    pub default_value: Option<String>,
    pub comment: Option<String>,
    pub check_constraints: Vec<String>,
    pub references: Option<Reference>,
}

impl Column {
    /// Create a new column with the given name and data type
    pub fn new(name: impl Into<String>, data_type: DataType) -> Self {
        Self {
            name: name.into(),
            data_type,
            is_primary_key: false,
            is_nullable: true,
            is_unique: false,
            default_value: None,
            comment: None,
            check_constraints: Vec::new(),
            references: None,
        }
    }

    /// Mark column as primary key
    pub fn primary_key(mut self) -> Self {
        self.is_primary_key = true;
        self.is_nullable = false; // Primary keys can't be null
        self
    }

    /// Mark column as not nullable
    pub fn not_null(mut self) -> Self {
        self.is_nullable = false;
        self
    }

    /// Mark column as unique
    pub fn unique(mut self) -> Self {
        self.is_unique = true;
        self
    }

    /// Set a default value for the column
    pub fn default(mut self, value: impl Into<String>) -> Self {
        self.default_value = Some(value.into());
        self
    }

    /// Add a comment to the column
    pub fn comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }

    /// Add a check constraint to the column
    pub fn check(mut self, constraint: impl Into<String>) -> Self {
        self.check_constraints.push(constraint.into());
        self
    }

    /// Add a foreign key reference
    pub fn references(mut self, table: impl Into<String>, column: impl Into<String>) -> Self {
        self.references = Some(Reference {
            table: table.into(),
            column: column.into(),
            on_delete: ReferentialAction::NoAction,
            on_update: ReferentialAction::NoAction,
        });
        self
    }
}

/// Represents a database constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub name: String,
    pub constraint_type: ConstraintType,
}

impl Constraint {
    /// Create a new constraint
    pub fn new(name: impl Into<String>, constraint_type: ConstraintType) -> Self {
        Self {
            name: name.into(),
            constraint_type,
        }
    }
}

/// Types of constraints in a database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstraintType {
    /// Primary key constraint
    PrimaryKey(Vec<String>),
    /// Foreign key constraint
    ForeignKey {
        columns: Vec<String>,
        referenced_table: String,
        referenced_columns: Vec<String>,
        on_delete: ReferentialAction,
        on_update: ReferentialAction,
    },
    /// Unique constraint
    Unique(Vec<String>),
    /// Check constraint
    Check(String),
}

/// Represents a database index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Index {
    pub name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
    pub method: Option<String>,
}

impl Index {
    /// Create a new index
    pub fn new(name: impl Into<String>, columns: Vec<String>) -> Self {
        Self {
            name: name.into(),
            columns,
            is_unique: false,
            method: None,
        }
    }

    /// Mark index as unique
    pub fn unique(mut self) -> Self {
        self.is_unique = true;
        self
    }

    /// Set the index method (e.g., "btree", "hash", "gin", etc.)
    pub fn method(mut self, method: impl Into<String>) -> Self {
        self.method = Some(method.into());
        self
    }
}

/// Represents a foreign key reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    pub table: String,
    pub column: String,
    pub on_delete: ReferentialAction,
    pub on_update: ReferentialAction,
}

/// Types of referential actions (ON DELETE, ON UPDATE)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferentialAction {
    NoAction,
    Restrict,
    Cascade,
    SetNull,
    SetDefault,
}

/// Supported data types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
    // Numeric types
    SmallInt,
    Integer,
    BigInt,
    Decimal { precision: Option<u8>, scale: Option<u8> },
    Numeric { precision: Option<u8>, scale: Option<u8> },
    Real,
    DoublePrecision,
    
    // Character types
    Char(Option<u32>),
    VarChar(Option<u32>),
    Text,
    
    // Binary types
    ByteA,
    
    // Date/Time types
    Date,
    Time,
    Timestamp,
    TimestampTZ,
    
    // Boolean type
    Boolean,
    
    // UUID type
    UUID,
    
    // JSON types
    JSON,
    JSONB,
    
    // Array type
    Array(Box<DataType>),
    
    // Custom type
    Custom(String),
}

impl std::fmt::Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataType::SmallInt => write!(f, "SMALLINT"),
            DataType::Integer => write!(f, "INTEGER"),
            DataType::BigInt => write!(f, "BIGINT"),
            DataType::Decimal { precision, scale } => {
                match (precision, scale) {
                    (Some(p), Some(s)) => write!(f, "DECIMAL({}, {})", p, s),
                    (Some(p), None) => write!(f, "DECIMAL({})", p),
                    _ => write!(f, "DECIMAL"),
                }
            }
            DataType::Numeric { precision, scale } => {
                match (precision, scale) {
                    (Some(p), Some(s)) => write!(f, "NUMERIC({}, {})", p, s),
                    (Some(p), None) => write!(f, "NUMERIC({})", p),
                    _ => write!(f, "NUMERIC"),
                }
            }
            DataType::Real => write!(f, "REAL"),
            DataType::DoublePrecision => write!(f, "DOUBLE PRECISION"),
            DataType::Char(Some(size)) => write!(f, "CHAR({})", size),
            DataType::Char(None) => write!(f, "CHAR"),
            DataType::VarChar(Some(size)) => write!(f, "VARCHAR({})", size),
            DataType::VarChar(None) => write!(f, "VARCHAR"),
            DataType::Text => write!(f, "TEXT"),
            DataType::ByteA => write!(f, "BYTEA"),
            DataType::Date => write!(f, "DATE"),
            DataType::Time => write!(f, "TIME"),
            DataType::Timestamp => write!(f, "TIMESTAMP"),
            DataType::TimestampTZ => write!(f, "TIMESTAMPTZ"),
            DataType::Boolean => write!(f, "BOOLEAN"),
            DataType::UUID => write!(f, "UUID"),
            DataType::JSON => write!(f, "JSON"),
            DataType::JSONB => write!(f, "JSONB"),
            DataType::Array(inner) => write!(f, "{}[]", inner),
            DataType::Custom(name) => write!(f, "{}", name),
        }
    }
} 