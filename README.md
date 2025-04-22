# Kohori

A Rust-based schema modeling library that generates database migrations, inspired by Drizzle ORM. Kohori provides first-class support for PostgreSQL Row-Level Security (RLS) policies.

## Features

- Define database schemas using native Rust code
- Generate SQL migrations from schema changes
- First-class support for PostgreSQL Row-Level Security (RLS) policies
- Type-safe RLS policy definitions embedded in schema models
- Migration versioning and tracking

## Example

```rust
use kohori::schema::{Table, Column, DataType};
use kohori::rls::{Policy, PolicyTarget, SecurityContext};

// Define a table with RLS policies
let users = Table::new("users")
    .column(Column::new("id", DataType::UUID).primary_key().default("gen_random_uuid()"))
    .column(Column::new("username", DataType::Text).not_null().unique())
    .column(Column::new("email", DataType::Text).not_null())
    .column(Column::new("created_at", DataType::Timestamp).not_null().default("now()"))
    .rls_policy(
        Policy::new("users_select_policy")
            .using("auth.uid() = id")
            .target(PolicyTarget::Select)
            .security_context(SecurityContext::Authenticated)
    );

// Generate migration SQL
let migration = schema.generate_migration("create_users_table");
println!("{}", migration.sql());
```

## Installation

Add Kohori to your Cargo.toml:

```toml
[dependencies]
kohori = "0.1.0"
```

## Documentation

For more detailed documentation, see the [API documentation](https://docs.rs/kohori).

## Roadmap

### Current Status (v0.1.0)
- ✅ Core schema modeling (tables, columns, constraints)
- ✅ PostgreSQL dialect support
- ✅ Row-Level Security (RLS) policy definitions
- ✅ Basic migration generation
- ✅ Migration file management

### Short-term Goals (v0.2.0)
- 🔲 Schema diffing for automated migration generation
- 🔲 Migration history tracking
- 🔲 Migration up/down operations
- 🔲 Command-line interface (CLI) for migration management
- 🔲 Documentation improvements and examples

### Mid-term Goals (v0.3.0)
- 🔲 Schema validation and integrity checks
- 🔲 Additional PostgreSQL features (extensions, functions, triggers)
- 🔲 Database connection management for applying migrations
- 🔲 Comprehensive test suite with real database integration
- 🔲 Support for additional constraints and column types

### Long-term Goals (v1.0.0)
- 🔲 MySQL dialect support
- 🔲 SQLite dialect support
- 🔲 SQL Server dialect support
- 🔲 GraphQL schema generation
- 🔲 TypeScript/JavaScript type definitions export
- 🔲 Integration with ORM libraries
- 🔲 Performance optimizations for large schemas

### Community Goals
- 🔲 Documentation website
- 🔲 Contribution guidelines
- 🔲 Example projects and templates
- 🔲 Community extensions and plugins

## License

This project is licensed under the MIT License - see the LICENSE file for details. 