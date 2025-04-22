use crate::schema::{Table, ConstraintType};
use crate::rls::Policy;
use crate::migration::{TableChange, ColumnChange};

/// A database dialect
pub trait Dialect: std::fmt::Debug {
    /// Generate SQL to create a table
    fn create_table(&self, table: &Table) -> String;
    
    /// Generate SQL to alter a table
    fn alter_table(&self, table: &str, changes: &[TableChange]) -> String;
    
    /// Generate SQL to drop a table
    fn drop_table(&self, table: &str, cascade: bool) -> String;
    
    /// Generate SQL to create an RLS policy
    fn create_rls_policy(&self, table: &str, policy: &Policy) -> String;
    
    /// Generate SQL to drop an RLS policy
    fn drop_rls_policy(&self, table: &str, policy: &str) -> String;
    
    /// Generate SQL to enable RLS on a table
    fn enable_rls(&self, table: &str, force: bool) -> String;
    
    /// Generate SQL to disable RLS on a table
    fn disable_rls(&self, table: &str) -> String;
}

/// The PostgreSQL dialect
#[derive(Debug, Default)]
pub struct PostgreSQL {
    /// Schema name (default: "public")
    pub schema: String,
}

impl PostgreSQL {
    /// Create a new PostgreSQL dialect with the default schema
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create a new PostgreSQL dialect with the given schema
    pub fn with_schema(schema: impl Into<String>) -> Self {
        Self {
            schema: schema.into(),
        }
    }
    
    /// Get the fully qualified table name (schema.table)
    fn qualified_table_name(&self, table: &str) -> String {
        if self.schema.is_empty() || self.schema == "public" {
            table.to_string()
        } else {
            format!("{}.{}", self.schema, table)
        }
    }
}

impl Dialect for PostgreSQL {
    fn create_table(&self, table: &Table) -> String {
        let mut sql = format!("CREATE TABLE {} (\n", self.qualified_table_name(&table.name));
        
        // Add columns
        let column_defs: Vec<String> = table.columns.iter().map(|col| {
            let mut def = format!("    {} {}", col.name, col.data_type);
            
            if col.is_primary_key {
                def.push_str(" PRIMARY KEY");
            }
            
            if !col.is_nullable && !col.is_primary_key {
                def.push_str(" NOT NULL");
            }
            
            if col.is_unique && !col.is_primary_key {
                def.push_str(" UNIQUE");
            }
            
            if let Some(default) = &col.default_value {
                def.push_str(&format!(" DEFAULT {}", default));
            }
            
            if let Some(reference) = &col.references {
                def.push_str(&format!(" REFERENCES {}({})",
                    self.qualified_table_name(&reference.table),
                    reference.column));
                    
                match reference.on_delete {
                    crate::schema::ReferentialAction::NoAction => {},
                    crate::schema::ReferentialAction::Restrict => {
                        def.push_str(" ON DELETE RESTRICT");
                    },
                    crate::schema::ReferentialAction::Cascade => {
                        def.push_str(" ON DELETE CASCADE");
                    },
                    crate::schema::ReferentialAction::SetNull => {
                        def.push_str(" ON DELETE SET NULL");
                    },
                    crate::schema::ReferentialAction::SetDefault => {
                        def.push_str(" ON DELETE SET DEFAULT");
                    },
                }
                
                match reference.on_update {
                    crate::schema::ReferentialAction::NoAction => {},
                    crate::schema::ReferentialAction::Restrict => {
                        def.push_str(" ON UPDATE RESTRICT");
                    },
                    crate::schema::ReferentialAction::Cascade => {
                        def.push_str(" ON UPDATE CASCADE");
                    },
                    crate::schema::ReferentialAction::SetNull => {
                        def.push_str(" ON UPDATE SET NULL");
                    },
                    crate::schema::ReferentialAction::SetDefault => {
                        def.push_str(" ON UPDATE SET DEFAULT");
                    },
                }
            }
            
            for check in &col.check_constraints {
                def.push_str(&format!(" CHECK ({})", check));
            }
            
            def
        }).collect();
        
        // Add constraints
        let mut constraint_defs = Vec::new();
        for constraint in &table.constraints {
            match &constraint.constraint_type {
                ConstraintType::PrimaryKey(columns) => {
                    constraint_defs.push(format!("    CONSTRAINT {} PRIMARY KEY ({})",
                        constraint.name,
                        columns.join(", ")));
                },
                ConstraintType::ForeignKey { columns, referenced_table, referenced_columns, on_delete, on_update } => {
                    let mut def = format!("    CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {}({})",
                        constraint.name,
                        columns.join(", "),
                        self.qualified_table_name(referenced_table),
                        referenced_columns.join(", "));
                        
                    match on_delete {
                        crate::schema::ReferentialAction::NoAction => {},
                        crate::schema::ReferentialAction::Restrict => {
                            def.push_str(" ON DELETE RESTRICT");
                        },
                        crate::schema::ReferentialAction::Cascade => {
                            def.push_str(" ON DELETE CASCADE");
                        },
                        crate::schema::ReferentialAction::SetNull => {
                            def.push_str(" ON DELETE SET NULL");
                        },
                        crate::schema::ReferentialAction::SetDefault => {
                            def.push_str(" ON DELETE SET DEFAULT");
                        },
                    }
                    
                    match on_update {
                        crate::schema::ReferentialAction::NoAction => {},
                        crate::schema::ReferentialAction::Restrict => {
                            def.push_str(" ON UPDATE RESTRICT");
                        },
                        crate::schema::ReferentialAction::Cascade => {
                            def.push_str(" ON UPDATE CASCADE");
                        },
                        crate::schema::ReferentialAction::SetNull => {
                            def.push_str(" ON UPDATE SET NULL");
                        },
                        crate::schema::ReferentialAction::SetDefault => {
                            def.push_str(" ON UPDATE SET DEFAULT");
                        },
                    }
                    
                    constraint_defs.push(def);
                },
                ConstraintType::Unique(columns) => {
                    constraint_defs.push(format!("    CONSTRAINT {} UNIQUE ({})",
                        constraint.name,
                        columns.join(", ")));
                },
                ConstraintType::Check(expr) => {
                    constraint_defs.push(format!("    CONSTRAINT {} CHECK ({})",
                        constraint.name,
                        expr));
                },
            }
        }
        
        // Combine column and constraint definitions
        let all_defs = [column_defs, constraint_defs].concat();
        sql.push_str(&all_defs.join(",\n"));
        sql.push_str("\n);\n");
        
        // Add table comments
        if let Some(comment) = &table.comment {
            sql.push_str(&format!("\nCOMMENT ON TABLE {} IS '{}';\n",
                self.qualified_table_name(&table.name),
                comment.replace('\'', "''")));
        }
        
        // Add column comments
        for column in &table.columns {
            if let Some(comment) = &column.comment {
                sql.push_str(&format!("\nCOMMENT ON COLUMN {}.{} IS '{}';\n",
                    self.qualified_table_name(&table.name),
                    column.name,
                    comment.replace('\'', "''")));
            }
        }
        
        // Add indexes
        for index in &table.indexes {
            let mut index_sql = format!("\nCREATE");
            
            if index.is_unique {
                index_sql.push_str(" UNIQUE");
            }
            
            index_sql.push_str(&format!(" INDEX {} ON {}",
                index.name,
                self.qualified_table_name(&table.name)));
                
            if let Some(method) = &index.method {
                index_sql.push_str(&format!(" USING {}", method));
            }
            
            index_sql.push_str(&format!(" ({});\n", index.columns.join(", ")));
            
            sql.push_str(&index_sql);
        }
        
        // Enable RLS if needed
        if table.rls_enabled {
            sql.push_str(&format!("\nALTER TABLE {} ENABLE ROW LEVEL SECURITY;\n",
                self.qualified_table_name(&table.name)));
        }
        
        // Add RLS policies
        for policy in &table.rls_policies {
            sql.push_str(&format!("\n{}\n", 
                crate::rls::generate_policy_sql(&self.qualified_table_name(&table.name), policy)));
        }
        
        sql
    }
    
    fn alter_table(&self, table: &str, changes: &[TableChange]) -> String {
        let qualified_table = self.qualified_table_name(table);
        let mut sql = String::new();
        
        for change in changes {
            match change {
                TableChange::AddColumn(column) => {
                    let mut col_def = format!("ALTER TABLE {} ADD COLUMN {} {}", 
                        qualified_table, column.name, column.data_type);
                    
                    if !column.is_nullable {
                        col_def.push_str(" NOT NULL");
                    }
                    
                    if let Some(default) = &column.default_value {
                        col_def.push_str(&format!(" DEFAULT {}", default));
                    }
                    
                    if column.is_unique {
                        col_def.push_str(" UNIQUE");
                    }
                    
                    if let Some(reference) = &column.references {
                        col_def.push_str(&format!(" REFERENCES {}({})",
                            self.qualified_table_name(&reference.table),
                            reference.column));
                            
                        match reference.on_delete {
                            crate::schema::ReferentialAction::NoAction => {},
                            crate::schema::ReferentialAction::Restrict => {
                                col_def.push_str(" ON DELETE RESTRICT");
                            },
                            crate::schema::ReferentialAction::Cascade => {
                                col_def.push_str(" ON DELETE CASCADE");
                            },
                            crate::schema::ReferentialAction::SetNull => {
                                col_def.push_str(" ON DELETE SET NULL");
                            },
                            crate::schema::ReferentialAction::SetDefault => {
                                col_def.push_str(" ON DELETE SET DEFAULT");
                            },
                        }
                        
                        match reference.on_update {
                            crate::schema::ReferentialAction::NoAction => {},
                            crate::schema::ReferentialAction::Restrict => {
                                col_def.push_str(" ON UPDATE RESTRICT");
                            },
                            crate::schema::ReferentialAction::Cascade => {
                                col_def.push_str(" ON UPDATE CASCADE");
                            },
                            crate::schema::ReferentialAction::SetNull => {
                                col_def.push_str(" ON UPDATE SET NULL");
                            },
                            crate::schema::ReferentialAction::SetDefault => {
                                col_def.push_str(" ON UPDATE SET DEFAULT");
                            },
                        }
                    }
                    
                    col_def.push_str(";\n");
                    sql.push_str(&col_def);
                    
                    if let Some(comment) = &column.comment {
                        sql.push_str(&format!("COMMENT ON COLUMN {}.{} IS '{}';\n",
                            qualified_table,
                            column.name,
                            comment.replace('\'', "''")));
                    }
                },
                TableChange::DropColumn { name, cascade } => {
                    let mut drop_sql = format!("ALTER TABLE {} DROP COLUMN {}", qualified_table, name);
                    if *cascade {
                        drop_sql.push_str(" CASCADE");
                    }
                    drop_sql.push_str(";\n");
                    sql.push_str(&drop_sql);
                },
                TableChange::AlterColumn { name, change } => {
                    match change {
                        ColumnChange::SetDataType(data_type) => {
                            sql.push_str(&format!("ALTER TABLE {} ALTER COLUMN {} TYPE {};\n",
                                qualified_table, name, data_type));
                        },
                        ColumnChange::SetNotNull => {
                            sql.push_str(&format!("ALTER TABLE {} ALTER COLUMN {} SET NOT NULL;\n",
                                qualified_table, name));
                        },
                        ColumnChange::DropNotNull => {
                            sql.push_str(&format!("ALTER TABLE {} ALTER COLUMN {} DROP NOT NULL;\n",
                                qualified_table, name));
                        },
                        ColumnChange::SetDefault(value) => {
                            sql.push_str(&format!("ALTER TABLE {} ALTER COLUMN {} SET DEFAULT {};\n",
                                qualified_table, name, value));
                        },
                        ColumnChange::DropDefault => {
                            sql.push_str(&format!("ALTER TABLE {} ALTER COLUMN {} DROP DEFAULT;\n",
                                qualified_table, name));
                        },
                    }
                },
                TableChange::AddConstraint(constraint) => {
                    match &constraint.constraint_type {
                        ConstraintType::PrimaryKey(columns) => {
                            sql.push_str(&format!("ALTER TABLE {} ADD CONSTRAINT {} PRIMARY KEY ({});\n",
                                qualified_table, constraint.name, columns.join(", ")));
                        },
                        ConstraintType::ForeignKey { columns, referenced_table, referenced_columns, on_delete, on_update } => {
                            let mut fk_sql = format!("ALTER TABLE {} ADD CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {}({})",
                                qualified_table,
                                constraint.name,
                                columns.join(", "),
                                self.qualified_table_name(referenced_table),
                                referenced_columns.join(", "));
                                
                            match on_delete {
                                crate::schema::ReferentialAction::NoAction => {},
                                crate::schema::ReferentialAction::Restrict => {
                                    fk_sql.push_str(" ON DELETE RESTRICT");
                                },
                                crate::schema::ReferentialAction::Cascade => {
                                    fk_sql.push_str(" ON DELETE CASCADE");
                                },
                                crate::schema::ReferentialAction::SetNull => {
                                    fk_sql.push_str(" ON DELETE SET NULL");
                                },
                                crate::schema::ReferentialAction::SetDefault => {
                                    fk_sql.push_str(" ON DELETE SET DEFAULT");
                                },
                            }
                            
                            match on_update {
                                crate::schema::ReferentialAction::NoAction => {},
                                crate::schema::ReferentialAction::Restrict => {
                                    fk_sql.push_str(" ON UPDATE RESTRICT");
                                },
                                crate::schema::ReferentialAction::Cascade => {
                                    fk_sql.push_str(" ON UPDATE CASCADE");
                                },
                                crate::schema::ReferentialAction::SetNull => {
                                    fk_sql.push_str(" ON UPDATE SET NULL");
                                },
                                crate::schema::ReferentialAction::SetDefault => {
                                    fk_sql.push_str(" ON UPDATE SET DEFAULT");
                                },
                            }
                            
                            fk_sql.push_str(";\n");
                            sql.push_str(&fk_sql);
                        },
                        ConstraintType::Unique(columns) => {
                            sql.push_str(&format!("ALTER TABLE {} ADD CONSTRAINT {} UNIQUE ({});\n",
                                qualified_table, constraint.name, columns.join(", ")));
                        },
                        ConstraintType::Check(expr) => {
                            sql.push_str(&format!("ALTER TABLE {} ADD CONSTRAINT {} CHECK ({});\n",
                                qualified_table, constraint.name, expr));
                        },
                    }
                },
                TableChange::DropConstraint { name, cascade } => {
                    let mut drop_sql = format!("ALTER TABLE {} DROP CONSTRAINT {}", qualified_table, name);
                    if *cascade {
                        drop_sql.push_str(" CASCADE");
                    }
                    drop_sql.push_str(";\n");
                    sql.push_str(&drop_sql);
                },
                TableChange::RenameTable { new_name } => {
                    sql.push_str(&format!("ALTER TABLE {} RENAME TO {};\n", qualified_table, new_name));
                },
                TableChange::RenameColumn { old_name, new_name } => {
                    sql.push_str(&format!("ALTER TABLE {} RENAME COLUMN {} TO {};\n",
                        qualified_table, old_name, new_name));
                },
            }
        }
        
        sql
    }
    
    fn drop_table(&self, table: &str, cascade: bool) -> String {
        let mut sql = format!("DROP TABLE {}", self.qualified_table_name(table));
        if cascade {
            sql.push_str(" CASCADE");
        }
        sql.push_str(";\n");
        sql
    }
    
    fn create_rls_policy(&self, table: &str, policy: &Policy) -> String {
        crate::rls::generate_policy_sql(&self.qualified_table_name(table), policy)
    }
    
    fn drop_rls_policy(&self, table: &str, policy: &str) -> String {
        format!("DROP POLICY {} ON {};\n", policy, self.qualified_table_name(table))
    }
    
    fn enable_rls(&self, table: &str, force: bool) -> String {
        let mut sql = format!("ALTER TABLE {} ENABLE ROW LEVEL SECURITY;\n", 
            self.qualified_table_name(table));
            
        if force {
            sql.push_str(&format!("ALTER TABLE {} FORCE ROW LEVEL SECURITY;\n", 
                self.qualified_table_name(table)));
        }
        
        sql
    }
    
    fn disable_rls(&self, table: &str) -> String {
        format!("ALTER TABLE {} DISABLE ROW LEVEL SECURITY;\n", 
            self.qualified_table_name(table))
    }
} 