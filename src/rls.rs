use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Represents a Row Level Security policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub name: String,
    pub using_expr: Option<String>,
    pub check_expr: Option<String>,
    pub target: PolicyTarget,
    pub roles: Vec<String>,
    pub security_context: SecurityContext,
    pub comment: Option<String>,
    pub state: PolicyState,
}

impl Policy {
    /// Create a new RLS policy with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            using_expr: None,
            check_expr: None,
            target: PolicyTarget::All,
            roles: Vec::new(),
            security_context: SecurityContext::Public,
            comment: None,
            state: PolicyState::Enabled,
        }
    }

    /// Set the USING expression for the policy
    pub fn using(mut self, expr: impl Into<String>) -> Self {
        self.using_expr = Some(expr.into());
        self
    }

    /// Set the CHECK expression for the policy
    pub fn check(mut self, expr: impl Into<String>) -> Self {
        self.check_expr = Some(expr.into());
        self
    }

    /// Set the target operation for the policy
    pub fn target(mut self, target: PolicyTarget) -> Self {
        self.target = target;
        self
    }

    /// Add roles that this policy applies to
    pub fn roles(mut self, roles: Vec<String>) -> Self {
        self.roles = roles;
        self
    }

    /// Set the security context for the policy
    pub fn security_context(mut self, security_context: SecurityContext) -> Self {
        self.security_context = security_context;
        self
    }

    /// Add a comment to the policy
    pub fn comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }

    /// Set the policy state
    pub fn state(mut self, state: PolicyState) -> Self {
        self.state = state;
        self
    }
}

/// The SQL operations a policy can target
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyTarget {
    All,
    Select,
    Insert,
    Update,
    Delete,
}

impl std::fmt::Display for PolicyTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolicyTarget::All => write!(f, "ALL"),
            PolicyTarget::Select => write!(f, "SELECT"),
            PolicyTarget::Insert => write!(f, "INSERT"),
            PolicyTarget::Update => write!(f, "UPDATE"),
            PolicyTarget::Delete => write!(f, "DELETE"),
        }
    }
}

/// The security context a policy operates within
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityContext {
    /// Available to all users (default)
    Public,
    /// Available only to authenticated users
    Authenticated,
    /// Available only to specific roles
    Role(Vec<String>),
    /// Custom security predicate
    Custom(String),
}

/// The state of a policy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyState {
    /// Policy is enabled
    Enabled,
    /// Policy is disabled
    Disabled,
    /// Policy is in testing mode (for migration transitions)
    Testing,
}

/// A container for RLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RLSConfig {
    pub enabled: bool,
    pub force_all_tables: bool,
    pub default_policies: Vec<Policy>,
    pub role_mappings: HashMap<String, String>,
}

impl Default for RLSConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            force_all_tables: false,
            default_policies: Vec::new(),
            role_mappings: HashMap::new(),
        }
    }
}

/// Generate SQL for an RLS policy
pub fn generate_policy_sql(table_name: &str, policy: &Policy) -> String {
    let mut sql = format!("CREATE POLICY {} ON {}", policy.name, table_name);
    
    // Add target if not ALL
    if policy.target != PolicyTarget::All {
        sql.push_str(&format!(" FOR {}", policy.target));
    }
    
    // Add roles if specified
    if !policy.roles.is_empty() {
        sql.push_str(&format!(" TO {}", policy.roles.join(", ")));
    }
    
    // Add USING expression if present
    if let Some(using_expr) = &policy.using_expr {
        sql.push_str(&format!(" USING ({})", using_expr));
    }
    
    // Add CHECK expression if present
    if let Some(check_expr) = &policy.check_expr {
        sql.push_str(&format!(" WITH CHECK ({})", check_expr));
    }
    
    sql.push(';');
    
    // Add comment if present
    if let Some(comment) = &policy.comment {
        sql.push_str(&format!("\nCOMMENT ON POLICY {} ON {} IS '{}';\n", 
            policy.name, table_name, comment.replace('\'', "''")));
    }
    
    // Handle policy state
    match policy.state {
        PolicyState::Enabled => {},  // Already enabled by default
        PolicyState::Disabled => {
            sql.push_str(&format!("\nALTER POLICY {} ON {} DISABLE;\n", policy.name, table_name));
        },
        PolicyState::Testing => {
            // For testing, we'll add a comment noting it's in testing mode
            sql.push_str(&format!("\n-- Policy {} is in TESTING mode\n", policy.name));
        }
    }
    
    sql
}

/// Generate SQL to enable RLS on a table
pub fn generate_enable_rls_sql(table_name: &str) -> String {
    format!("ALTER TABLE {} ENABLE ROW LEVEL SECURITY;", table_name)
}

/// Generate SQL to force RLS on a table (even for table owners)
pub fn generate_force_rls_sql(table_name: &str) -> String {
    format!("ALTER TABLE {} FORCE ROW LEVEL SECURITY;", table_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_builder() {
        let policy = Policy::new("test_policy")
            .using("user_id = auth.uid()")
            .target(PolicyTarget::Select)
            .security_context(SecurityContext::Authenticated)
            .comment("Test policy for authenticated users");
        
        assert_eq!(policy.name, "test_policy");
        assert_eq!(policy.using_expr, Some("user_id = auth.uid()".to_string()));
        assert_eq!(policy.target, PolicyTarget::Select);
        assert_eq!(policy.security_context, SecurityContext::Authenticated);
        assert_eq!(policy.comment, Some("Test policy for authenticated users".to_string()));
    }

    #[test]
    fn test_generate_policy_sql() {
        let policy = Policy::new("test_policy")
            .using("user_id = auth.uid()")
            .target(PolicyTarget::Select)
            .roles(vec!["authenticated".to_string()]);
        
        let sql = generate_policy_sql("users", &policy);
        assert!(sql.contains("CREATE POLICY test_policy ON users"));
        assert!(sql.contains("FOR SELECT"));
        assert!(sql.contains("TO authenticated"));
        assert!(sql.contains("USING (user_id = auth.uid())"));
    }
} 