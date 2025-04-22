use kohori::schema::{Table, Column, DataType, Index, Constraint, ConstraintType};
use kohori::rls::{Policy, PolicyTarget, SecurityContext};
use kohori::{Schema, Result};
use kohori::migration::{MigrationManager, MigrationPlan, MigrationOperation};
use kohori::dialect::PostgreSQL;
use std::path::PathBuf;

fn main() -> Result<()> {
    // Original schema (simulating the "current" schema in the database)
    let mut original_schema = Schema::new("public");
    
    // Define a users table
    let users_v1 = Table::new("users")
        .column(Column::new("id", DataType::UUID).primary_key().default("gen_random_uuid()"))
        .column(Column::new("email", DataType::Text).not_null().unique())
        .column(Column::new("name", DataType::VarChar(Some(255))).not_null())
        .column(Column::new("created_at", DataType::TimestampTZ).not_null().default("now()"))
        .enable_rls()
        // Basic RLS policy
        .rls_policy(
            Policy::new("users_select_policy")
                .using("auth.uid() = id")
                .target(PolicyTarget::Select)
                .security_context(SecurityContext::Authenticated)
        )
        // Add an index directly to the table
        .index(Index::new("users_email_idx", vec!["email".to_string()]));
    
    // Define a products table
    let products_v1 = Table::new("products")
        .column(Column::new("id", DataType::UUID).primary_key().default("gen_random_uuid()"))
        .column(Column::new("name", DataType::VarChar(Some(255))).not_null())
        .column(Column::new("description", DataType::Text))
        .column(Column::new("price", DataType::Numeric { precision: Some(10), scale: Some(2) }).not_null())
        .column(Column::new("created_at", DataType::TimestampTZ).not_null().default("now()"));
    
    // Add tables to original schema
    original_schema.add_table(users_v1).add_table(products_v1);
    
    println!("Original schema created with {} tables", original_schema.tables.len());
    
    // Updated schema with changes
    let mut updated_schema = Schema::new("public");
    
    // Define an updated users table with additional columns and changed policies
    let users_v2 = Table::new("users")
        .column(Column::new("id", DataType::UUID).primary_key().default("gen_random_uuid()"))
        .column(Column::new("email", DataType::Text).not_null().unique())
        .column(Column::new("name", DataType::VarChar(Some(255))).not_null())
        // Add a new nickname column
        .column(Column::new("nickname", DataType::VarChar(Some(100))))
        // Add a role column with a check constraint
        .column(Column::new("role", DataType::VarChar(Some(50))).not_null().default("'user'")
            .check("role IN ('user', 'admin', 'moderator')"))
        .column(Column::new("created_at", DataType::TimestampTZ).not_null().default("now()"))
        // Add updated_at column
        .column(Column::new("updated_at", DataType::TimestampTZ).not_null().default("now()"))
        .enable_rls()
        // Keep the original policy
        .rls_policy(
            Policy::new("users_select_policy")
                .using("auth.uid() = id")
                .target(PolicyTarget::Select)
                .security_context(SecurityContext::Authenticated)
        )
        // Add more complex policies
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
        )
        // Add the existing index (will be preserved in the migration)
        .index(Index::new("users_email_idx", vec!["email".to_string()]))
        // Add a new index on name
        .index(Index::new("users_name_idx", vec!["name".to_string()]));
    
    // Updated products table
    let products_v2 = Table::new("products")
        .column(Column::new("id", DataType::UUID).primary_key().default("gen_random_uuid()"))
        .column(Column::new("name", DataType::VarChar(Some(255))).not_null())
        .column(Column::new("description", DataType::Text))
        .column(Column::new("price", DataType::Numeric { precision: Some(10), scale: Some(2) }).not_null())
        // Add stock column
        .column(Column::new("stock", DataType::Integer).not_null().default("0"))
        // Add category column
        .column(Column::new("category_id", DataType::UUID).references("categories", "id"))
        .column(Column::new("created_at", DataType::TimestampTZ).not_null().default("now()"))
        .column(Column::new("updated_at", DataType::TimestampTZ).not_null().default("now()"))
        // Enable RLS on products
        .enable_rls()
        .rls_policy(
            Policy::new("products_read_policy")
                .using("true")
                .target(PolicyTarget::Select)
                .security_context(SecurityContext::Public)
                .comment("Anyone can read products")
        )
        .rls_policy(
            Policy::new("products_write_policy")
                .using("auth.role() IN ('admin', 'editor')")
                .target(PolicyTarget::All)
                .security_context(SecurityContext::Role(vec!["admin".to_string(), "editor".to_string()]))
                .comment("Admins and editors can manage products")
        )
        // Add an index on category_id
        .index(Index::new("products_category_idx", vec!["category_id".to_string()]));
    
    // Add a completely new table - categories
    let categories = Table::new("categories")
        .column(Column::new("id", DataType::UUID).primary_key().default("gen_random_uuid()"))
        .column(Column::new("name", DataType::VarChar(Some(100))).not_null().unique())
        .column(Column::new("description", DataType::Text))
        .column(Column::new("parent_id", DataType::UUID).references("categories", "id"))
        .column(Column::new("created_at", DataType::TimestampTZ).not_null().default("now()"))
        .column(Column::new("updated_at", DataType::TimestampTZ).not_null().default("now()"))
        // Add a unique constraint
        .constraint(Constraint::new(
            "categories_name_unique", 
            ConstraintType::Unique(vec!["name".to_string()])
        ));
    
    // Add a completely new table - orders
    let orders = Table::new("orders")
        .column(Column::new("id", DataType::UUID).primary_key().default("gen_random_uuid()"))
        .column(Column::new("user_id", DataType::UUID).not_null().references("users", "id"))
        .column(Column::new("total_amount", DataType::Numeric { precision: Some(12), scale: Some(2) }).not_null())
        .column(Column::new("status", DataType::VarChar(Some(50))).not_null().default("'pending'"))
        .column(Column::new("created_at", DataType::TimestampTZ).not_null().default("now()"))
        .column(Column::new("updated_at", DataType::TimestampTZ).not_null().default("now()"))
        .enable_rls()
        .rls_policy(
            Policy::new("orders_owner_policy")
                .using("auth.uid() = user_id")
                .target(PolicyTarget::All)
                .security_context(SecurityContext::Authenticated)
                .comment("Users can only see and modify their own orders")
        )
        .rls_policy(
            Policy::new("orders_admin_policy")
                .using("auth.role() = 'admin'")
                .target(PolicyTarget::All)
                .security_context(SecurityContext::Role(vec!["admin".to_string()]))
                .comment("Admins can see all orders")
        )
        // Add a composite index on user_id and status
        .index(Index::new("orders_user_status_idx", vec!["user_id".to_string(), "status".to_string()]));
    
    // Add a completely new table - order_items
    let order_items = Table::new("order_items")
        .column(Column::new("id", DataType::UUID).primary_key().default("gen_random_uuid()"))
        .column(Column::new("order_id", DataType::UUID).not_null().references("orders", "id"))
        .column(Column::new("product_id", DataType::UUID).not_null().references("products", "id"))
        .column(Column::new("quantity", DataType::Integer).not_null())
        .column(Column::new("price", DataType::Numeric { precision: Some(10), scale: Some(2) }).not_null())
        .column(Column::new("created_at", DataType::TimestampTZ).not_null().default("now()"))
        // Add a foreign key constraint
        .constraint(Constraint::new(
            "order_items_order_fk",
            ConstraintType::ForeignKey {
                columns: vec!["order_id".to_string()],
                referenced_table: "orders".to_string(),
                referenced_columns: vec!["id".to_string()],
                on_delete: kohori::schema::ReferentialAction::Cascade,
                on_update: kohori::schema::ReferentialAction::NoAction,
            }
        ));
    
    // Add tables to updated schema
    updated_schema
        .add_table(users_v2)
        .add_table(products_v2)
        .add_table(categories)
        .add_table(orders)
        .add_table(order_items);
    
    // Add custom SQL functions (using raw SQL operations)
    let function_sql = "
    CREATE OR REPLACE FUNCTION update_updated_at()
    RETURNS TRIGGER AS $$
    BEGIN
        NEW.updated_at = now();
        RETURN NEW;
    END;
    $$ LANGUAGE plpgsql;
    ";
    
    let create_function_op = MigrationOperation::RawSQL {
        sql: function_sql.to_string(),
        description: Some("Create updated_at trigger function".to_string()),
    };
    
    // Create triggers for updated_at columns
    let users_trigger_sql = "
    CREATE TRIGGER set_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();
    ";
    
    let products_trigger_sql = "
    CREATE TRIGGER set_products_updated_at
    BEFORE UPDATE ON products
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();
    ";
    
    let orders_trigger_sql = "
    CREATE TRIGGER set_orders_updated_at
    BEFORE UPDATE ON orders
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();
    ";
    
    let categories_trigger_sql = "
    CREATE TRIGGER set_categories_updated_at
    BEFORE UPDATE ON categories
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();
    ";
    
    let create_users_trigger_op = MigrationOperation::RawSQL {
        sql: users_trigger_sql.to_string(),
        description: Some("Create users updated_at trigger".to_string()),
    };
    
    let create_products_trigger_op = MigrationOperation::RawSQL {
        sql: products_trigger_sql.to_string(),
        description: Some("Create products updated_at trigger".to_string()),
    };
    
    let create_orders_trigger_op = MigrationOperation::RawSQL {
        sql: orders_trigger_sql.to_string(),
        description: Some("Create orders updated_at trigger".to_string()),
    };
    
    let create_categories_trigger_op = MigrationOperation::RawSQL {
        sql: categories_trigger_sql.to_string(),
        description: Some("Create categories updated_at trigger".to_string()),
    };
    
    println!("Updated schema created with {} tables", updated_schema.tables.len());
    
    // Create a dialect for SQL generation
    let dialect = Box::new(PostgreSQL::default());
    
    // Create a migration manager
    let migrations_dir = PathBuf::from("./migrations");
    let manager = MigrationManager::new(migrations_dir, dialect);
    
    // Generate the migration operations by diffing the schemas
    let mut operations = manager.diff_schemas(&original_schema, &updated_schema);
    
    println!("Generated {} migration operations from schema diff", operations.len());
    
    // Add our custom function and trigger operations
    operations.push(create_function_op);
    operations.push(create_users_trigger_op);
    operations.push(create_products_trigger_op);
    operations.push(create_orders_trigger_op);
    operations.push(create_categories_trigger_op);
    
    println!("Total {} migration operations after adding custom operations", operations.len());
    
    // Create a migration plan
    let mut plan = MigrationPlan::new("complex_schema_update");
    
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