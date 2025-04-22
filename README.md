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

## License

This project is licensed under the MIT License - see the LICENSE file for details. 