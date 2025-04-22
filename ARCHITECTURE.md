# Kohori Architecture

Kohori is a Rust-based schema modeling library that generates database migrations with first-class support for PostgreSQL Row-Level Security (RLS) policies.

## Project Structure

```
kohori/
├── Cargo.toml              # Project configuration and dependencies
├── examples/
│   └── simple.rs           # Example demonstrating schema definition and migration generation
├── src/
│   ├── lib.rs              # Main library entry point
│   ├── schema.rs           # Schema definition models (tables, columns, constraints)
│   ├── rls.rs              # Row-Level Security policy definitions
│   ├── migration.rs        # Migration generation and management
│   ├── dialect.rs          # SQL dialect implementations (PostgreSQL)
│   └── error.rs            # Error handling
└── migrations/             # Generated migration files
```

## Core Components

### Schema

The Schema component provides structures and traits for defining database schemas using native Rust code. It includes:

- Table definitions
- Column definitions with types and constraints
- Foreign key references
- Indexes

### Row-Level Security (RLS)

The RLS component provides first-class support for PostgreSQL Row-Level Security policies:

- Policy definitions with USING and CHECK expressions
- Policy targets (SELECT, INSERT, UPDATE, DELETE, ALL)
- Security contexts (Public, Authenticated, Role-based)
- Policy state management (Enabled, Disabled, Testing)

### Migration

The Migration component handles:

- Schema versioning
- Migration generation from schema definitions
- SQL file generation
- Migration tracking

### Dialect

The Dialect component provides SQL generation for different database systems:

- Currently supports PostgreSQL
- Extensible for other databases

## Core Data Flow

1. Define schema using Rust code
2. Add RLS policies to tables
3. Generate a migration plan
4. Convert the plan to a migration
5. Generate SQL for the migration
6. Write the SQL to a file or execute it against the database

## Usage Example

```rust
// Define schema
let mut schema = Schema::new("public");

// Define a table with RLS policies
let users = Table::new("users")
    .column(Column::new("id", DataType::UUID).primary_key().default("gen_random_uuid()"))
    .column(Column::new("email", DataType::Text).not_null())
    .enable_rls()
    .rls_policy(
        Policy::new("users_select_policy")
            .using("auth.uid() = id")
            .target(PolicyTarget::Select)
            .security_context(SecurityContext::Authenticated)
    );

// Add table to schema
schema.add_table(users);

// Generate migration
let migration = schema.generate_migration("create_users_table");

// Generate SQL
let sql = migration.to_sql(&PostgreSQL::default());

// Write to file
migration.write_to_file("./migrations")?;
``` 