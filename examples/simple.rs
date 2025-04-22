use kohori::schema::{Table, Column, DataType};
use kohori::rls::{Policy, PolicyTarget, SecurityContext};
use kohori::{Schema, Result};
use kohori::migration::{MigrationOperation, MigrationPlan};
use kohori::dialect::PostgreSQL;

fn main() -> Result<()> {
    // Create a schema
    let mut schema = Schema::new("public");
    
    // Define a users table with RLS policies
    let users = Table::new("users")
        .column(Column::new("id", DataType::UUID).primary_key().default("gen_random_uuid()"))
        .column(Column::new("username", DataType::VarChar(Some(255))).not_null().unique())
        .column(Column::new("email", DataType::Text).not_null())
        .column(Column::new("created_at", DataType::TimestampTZ).not_null().default("now()"))
        .column(Column::new("updated_at", DataType::TimestampTZ).not_null().default("now()"))
        .enable_rls()
        .rls_policy(
            Policy::new("users_select_policy")
                .using("auth.uid() = id")
                .target(PolicyTarget::Select)
                .security_context(SecurityContext::Authenticated)
                .comment("Users can only see their own profile")
        )
        .rls_policy(
            Policy::new("users_insert_policy")
                .using("auth.uid() = id")
                .target(PolicyTarget::Insert)
                .security_context(SecurityContext::Authenticated)
                .comment("Users can only insert their own profile")
        )
        .rls_policy(
            Policy::new("users_update_policy")
                .using("auth.uid() = id")
                .target(PolicyTarget::Update)
                .security_context(SecurityContext::Authenticated)
                .comment("Users can only update their own profile")
        )
        .rls_policy(
            Policy::new("users_delete_policy")
                .using("auth.uid() = id")
                .target(PolicyTarget::Delete)
                .security_context(SecurityContext::Authenticated)
                .comment("Users can only delete their own profile")
        )
        .rls_policy(
            Policy::new("admin_access_policy")
                .using("auth.role() = 'admin'")
                .target(PolicyTarget::All)
                .security_context(SecurityContext::Role(vec!["admin".to_string()]))
                .comment("Admins can do anything with any user")
        );
    
    // Define a posts table with RLS policies
    let posts = Table::new("posts")
        .column(Column::new("id", DataType::UUID).primary_key().default("gen_random_uuid()"))
        .column(Column::new("title", DataType::Text).not_null())
        .column(Column::new("content", DataType::Text).not_null())
        .column(Column::new("user_id", DataType::UUID).not_null()
            .references("users", "id"))
        .column(Column::new("published", DataType::Boolean).not_null().default("false"))
        .column(Column::new("created_at", DataType::TimestampTZ).not_null().default("now()"))
        .column(Column::new("updated_at", DataType::TimestampTZ).not_null().default("now()"))
        .enable_rls()
        .rls_policy(
            Policy::new("posts_select_public_policy")
                .using("published = true")
                .target(PolicyTarget::Select)
                .security_context(SecurityContext::Public)
                .comment("Anyone can see published posts")
        )
        .rls_policy(
            Policy::new("posts_select_own_policy")
                .using("auth.uid() = user_id")
                .target(PolicyTarget::Select)
                .security_context(SecurityContext::Authenticated)
                .comment("Users can see their own unpublished posts")
        )
        .rls_policy(
            Policy::new("posts_insert_own_policy")
                .using("auth.uid() = user_id")
                .target(PolicyTarget::Insert)
                .security_context(SecurityContext::Authenticated)
                .comment("Users can only insert their own posts")
        )
        .rls_policy(
            Policy::new("posts_update_own_policy")
                .using("auth.uid() = user_id")
                .target(PolicyTarget::Update)
                .security_context(SecurityContext::Authenticated)
                .comment("Users can only update their own posts")
        )
        .rls_policy(
            Policy::new("posts_delete_own_policy")
                .using("auth.uid() = user_id")
                .target(PolicyTarget::Delete)
                .security_context(SecurityContext::Authenticated)
                .comment("Users can only delete their own posts")
        )
        .rls_policy(
            Policy::new("admin_posts_policy")
                .using("auth.role() = 'admin'")
                .target(PolicyTarget::All)
                .security_context(SecurityContext::Role(vec!["admin".to_string()]))
                .comment("Admins can do anything with any post")
        );
    
    // Add tables to schema
    schema.add_table(users).add_table(posts);
    
    // Create a migration plan
    let mut plan = MigrationPlan::new("initial_schema");
    
    plan.add_operation(MigrationOperation::RawSQL {
        sql: "CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\";".to_string(),
        description: Some("Enable UUID extension".to_string()),
    });
    
    plan.add_operation(MigrationOperation::RawSQL {
        sql: "CREATE TABLE IF NOT EXISTS _kohori_migrations (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT now()
);".to_string(),
        description: Some("Create migrations table".to_string()),
    });
    
    plan.add_operation(MigrationOperation::CreateTable {
        table: schema.tables[0].clone(), // Users table
    });
    
    plan.add_operation(MigrationOperation::CreateTable {
        table: schema.tables[1].clone(), // Posts table
    });
    
    // Convert plan to migration
    let migration = plan.to_migration(schema);
    
    // Generate SQL
    let sql = migration.to_sql(&PostgreSQL::default());
    
    // Print SQL
    println!("{}", sql);
    
    Ok(())
} 