use kohori::schema::{Table, Column, DataType};
use kohori::rls::{Policy, PolicyTarget, SecurityContext};
use kohori::{Schema, Result};
use kohori::migration::{MigrationManager, MigrationPlan};
use kohori::dialect::PostgreSQL;
use std::path::PathBuf;

fn main() -> Result<()> {
    // Original schema (simulating the "current" schema in the database)
    let mut original_schema = Schema::new("public");
    
    // Define a users table
    let users_v1 = Table::new("users")
        .column(Column::new("id", DataType::UUID).primary_key().default("gen_random_uuid()"))
        .column(Column::new("email", DataType::Text).not_null())
        .column(Column::new("created_at", DataType::TimestampTZ).not_null().default("now()"))
        .enable_rls()
        .rls_policy(
            Policy::new("users_select_policy")
                .using("auth.uid() = id")
                .target(PolicyTarget::Select)
                .security_context(SecurityContext::Authenticated)
        );
    
    // Add table to original schema
    original_schema.add_table(users_v1);
    
    println!("Original schema created with {} tables", original_schema.tables.len());
    
    // Updated schema with changes
    let mut updated_schema = Schema::new("public");
    
    // Define an updated users table with additional columns and changed policies
    let users_v2 = Table::new("users")
        .column(Column::new("id", DataType::UUID).primary_key().default("gen_random_uuid()"))
        .column(Column::new("email", DataType::Text).not_null())
        // Added username column
        .column(Column::new("username", DataType::VarChar(Some(100))).not_null().default("'unnamed'"))
        // Added profile_picture column
        .column(Column::new("profile_picture", DataType::Text).default("NULL"))
        .column(Column::new("created_at", DataType::TimestampTZ).not_null().default("now()"))
        // Added updated_at column
        .column(Column::new("updated_at", DataType::TimestampTZ).not_null().default("now()"))
        .enable_rls()
        // Keep the original policy
        .rls_policy(
            Policy::new("users_select_policy")
                .using("auth.uid() = id")
                .target(PolicyTarget::Select)
                .security_context(SecurityContext::Authenticated)
        )
        // Add new policies
        .rls_policy(
            Policy::new("users_update_policy")
                .using("auth.uid() = id")
                .target(PolicyTarget::Update)
                .security_context(SecurityContext::Authenticated)
                .comment("Users can only update their own records")
        )
        .rls_policy(
            Policy::new("admin_users_policy")
                .using("auth.role() = 'admin'")
                .target(PolicyTarget::All)
                .security_context(SecurityContext::Role(vec!["admin".to_string()]))
                .comment("Admins can do anything with users")
        );
    
    // Add a completely new table - posts
    let posts = Table::new("posts")
        .column(Column::new("id", DataType::UUID).primary_key().default("gen_random_uuid()"))
        .column(Column::new("user_id", DataType::UUID).not_null().references("users", "id"))
        .column(Column::new("title", DataType::Text).not_null())
        .column(Column::new("content", DataType::Text).not_null())
        .column(Column::new("published", DataType::Boolean).not_null().default("false"))
        .column(Column::new("created_at", DataType::TimestampTZ).not_null().default("now()"))
        .column(Column::new("updated_at", DataType::TimestampTZ).not_null().default("now()"))
        .enable_rls()
        .rls_policy(
            Policy::new("posts_select_public_policy")
                .using("published = true")
                .target(PolicyTarget::Select)
                .security_context(SecurityContext::Public)
                .comment("Anyone can read published posts")
        )
        .rls_policy(
            Policy::new("posts_owner_policy")
                .using("auth.uid() = user_id")
                .target(PolicyTarget::All)
                .security_context(SecurityContext::Authenticated)
                .comment("Post owners can do anything with their posts")
        );
    
    // Add tables to updated schema
    updated_schema.add_table(users_v2).add_table(posts);
    
    println!("Updated schema created with {} tables", updated_schema.tables.len());
    
    // Create a dialect for SQL generation
    let dialect = Box::new(PostgreSQL::default());
    
    // Create a migration manager
    let migrations_dir = PathBuf::from("./migrations");
    let manager = MigrationManager::new(migrations_dir, dialect);
    
    // Generate the migration operations by diffing the schemas
    let operations = manager.diff_schemas(&original_schema, &updated_schema);
    
    println!("Generated {} migration operations", operations.len());
    
    // Create a migration plan
    let mut plan = MigrationPlan::new("schema_update");
    
    // Add all operations to the plan
    for operation in operations {
        plan.add_operation(operation);
    }
    
    // Convert plan to migration
    let migration = plan.to_migration(updated_schema.clone());
    
    // Save the migration to disk
    let migration_path = migration.write_to_file("./migrations")?;
    println!("Migration written to {}", migration_path.display());
    
    // Print SQL preview
    println!("\nSQL Preview (first 500 chars):");
    let sql = migration.to_sql(&PostgreSQL::default());
    if sql.len() > 500 {
        println!("{}...", &sql[..500]);
    } else {
        println!("{}", sql);
    }
    
    Ok(())
} 