// src/security/access_control.rs
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AccessLevel {
    /// 只读权限
    ReadOnly,
    /// 读写权限
    ReadWrite,
    /// 管理权限
    Admin,
    /// 超级管理员权限
    SuperAdmin,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ResourceType {
    /// 实例资源
    Instance,
    /// 记忆资源
    Memory,
    /// API 资源
    Api,
    /// 配置资源
    Config,
    /// 文件资源
    File,
    /// 日志资源
    Log,
    /// 技能资源
    Skill,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Action {
    /// 读取操作
    Read,
    /// 写入操作
    Write,
    /// 更新操作
    Update,
    /// 删除操作
    Delete,
    /// 执行操作
    Execute,
    /// 管理操作
    Manage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    pub resource_type: ResourceType,
    pub actions: HashSet<Action>,
    pub allowed_instances: HashSet<String>,  // 空表示允许所有实例
    pub denied_instances: HashSet<String>,   // 优先级高于 allowed_instances
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_until: Option<DateTime<Utc>>,
    pub condition: Option<String>,  // 可选的条件表达式
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub description: String,
    pub permissions: Vec<PermissionRule>,
    pub inherit_from: Vec<String>,  // 继承的角色
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub roles: Vec<String>,
    pub api_keys: Vec<ApiKey>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub key: String,
    pub name: String,
    pub scopes: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked: bool,
}

pub struct AccessControlManager {
    /// 角色定义
    roles: Arc<RwLock<HashMap<String, Role>>>,
    /// 用户定义
    users: Arc<RwLock<HashMap<String, User>>>,
    /// 权限缓存
    permission_cache: Arc<RwLock<HashMap<String, Vec<PermissionRule>>>>,
    /// 审计日志
    audit_logger: Arc<AuditLogger>,
}

impl AccessControlManager {
    pub fn new() -> Self {
        Self {
            roles: Arc::new(RwLock::new(Self::default_roles())),
            users: Arc::new(RwLock::new(HashMap::new())),
            permission_cache: Arc::new(RwLock::new(HashMap::new())),
            audit_logger: Arc::new(AuditLogger::new()),
        }
    }

    /// 创建默认角色
    fn default_roles() -> HashMap<String, Role> {
        let mut roles = HashMap::new();

        // 董事长角色 - 最高权限
        roles.insert("chairman".to_string(), Role {
            id: "chairman".to_string(),
            name: "董事长".to_string(),
            description: "系统最高权限角色，管理所有实例".to_string(),
            permissions: vec![
                PermissionRule {
                    resource_type: ResourceType::Instance,
                    actions: vec![Action::Read, Action::Write, Action::Update, Action::Delete, Action::Manage].into_iter().collect(),
                    allowed_instances: HashSet::new(),  // 允许所有实例
                    denied_instances: HashSet::new(),
                    valid_from: None,
                    valid_until: None,
                    condition: None,
                },
                PermissionRule {
                    resource_type: ResourceType::Memory,
                    actions: vec![Action::Read, Action::Write, Action::Delete].into_iter().collect(),
                    allowed_instances: HashSet::new(),
                    denied_instances: HashSet::new(),
                    valid_from: None,
                    valid_until: None,
                    condition: None,
                },
                PermissionRule {
                    resource_type: ResourceType::Config,
                    actions: vec![Action::Read, Action::Write, Action::Update].into_iter().collect(),
                    allowed_instances: HashSet::new(),
                    denied_instances: HashSet::new(),
                    valid_from: None,
                    valid_until: None,
                    condition: None,
                },
            ],
            inherit_from: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });

        // CEO 角色 - 管理自己的实例
        roles.insert("ceo".to_string(), Role {
            id: "ceo".to_string(),
            name: "CEO".to_string(),
            description: "公司实例管理者，管理自己创建的实例".to_string(),
            permissions: vec![
                PermissionRule {
                    resource_type: ResourceType::Instance,
                    actions: vec![Action::Read, Action::Write, Action::Update].into_iter().collect(),
                    allowed_instances: HashSet::new(),  // 通过策略动态确定
                    denied_instances: HashSet::new(),
                    valid_from: None,
                    valid_until: None,
                    condition: Some("is_owner".to_string()),  // 仅允许管理自己拥有的实例
                },
                PermissionRule {
                    resource_type: ResourceType::Memory,
                    actions: vec![Action::Read, Action::Write].into_iter().collect(),
                    allowed_instances: HashSet::new(),
                    denied_instances: HashSet::new(),
                    valid_from: None,
                    valid_until: None,
                    condition: Some("is_owner".to_string()),
                },
                PermissionRule {
                    resource_type: ResourceType::Api,
                    actions: vec![Action::Execute].into_iter().collect(),
                    allowed_instances: HashSet::new(),
                    denied_instances: HashSet::new(),
                    valid_from: None,
                    valid_until: None,
                    condition: Some("is_owner".to_string()),
                },
            ],
            inherit_from: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });

        // 团队负责人角色
        roles.insert("team_lead".to_string(), Role {
            id: "team_lead".to_string(),
            name: "团队负责人".to_string(),
            description: "团队管理者，管理自己团队的资源".to_string(),
            permissions: vec![
                PermissionRule {
                    resource_type: ResourceType::Memory,
                    actions: vec![Action::Read, Action::Write].into_iter().collect(),
                    allowed_instances: HashSet::new(),
                    denied_instances: HashSet::new(),
                    valid_from: None,
                    valid_until: None,
                    condition: Some("is_team_member".to_string()),
                },
                PermissionRule {
                    resource_type: ResourceType::Skill,
                    actions: vec![Action::Execute].into_iter().collect(),
                    allowed_instances: HashSet::new(),
                    denied_instances: HashSet::new(),
                    valid_from: None,
                    valid_until: None,
                    condition: Some("is_team_member".to_string()),
                },
            ],
            inherit_from: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });

        // 工作 Agent 角色
        roles.insert("worker".to_string(), Role {
            id: "worker".to_string(),
            name: "工作 Agent".to_string(),
            description: "执行具体任务的 Agent".to_string(),
            permissions: vec![
                PermissionRule {
                    resource_type: ResourceType::Memory,
                    actions: vec![Action::Read, Action::Write].into_iter().collect(),
                    allowed_instances: HashSet::new(),
                    denied_instances: HashSet::new(),
                    valid_from: None,
                    valid_until: None,
                    condition: Some("is_assigned_task".to_string()),
                },
                PermissionRule {
                    resource_type: ResourceType::Api,
                    actions: vec![Action::Execute].into_iter().collect(),
                    allowed_instances: HashSet::new(),
                    denied_instances: HashSet::new(),
                    valid_from: None,
                    valid_until: None,
                    condition: Some("is_assigned_task".to_string()),
                },
            ],
            inherit_from: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });

        roles
    }

    /// 检查权限
    pub async fn check_permission(
        &self,
        user_id: &str,
        instance_id: &str,
        resource_type: &ResourceType,
        action: &Action,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // 获取用户
        let user = {
            let users = self.users.read().await;
            users.get(user_id)
                .cloned()
                .ok_or("用户不存在")?
        };

        // 获取用户的权限规则
        let permission_rules = self.get_user_permissions(&user).await?;

        // 检查是否有相应权限
        for rule in &permission_rules {
            if rule.resource_type == *resource_type &&
               rule.actions.contains(action) &&
               self.is_instance_allowed(rule, instance_id) &&
               self.is_rule_valid(rule) &&
               self.evaluate_condition(rule, user_id, instance_id).await {
                return Ok(true);
            }
        }

        // 记录拒绝访问
        self.audit_logger.log_access_denied(
            user_id,
            instance_id,
            resource_type,
            action,
        ).await;

        Ok(false)
    }

    /// 检查 API 密钥权限
    pub async fn check_api_key_permission(
        &self,
        api_key: &str,
        instance_id: &str,
        resource_type: &ResourceType,
        action: &Action,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // 查找拥有该 API 密钥的用户
        let mut user_id = None;
        {
            let users = self.users.read().await;
            for (uid, user) in users.iter() {
                if user.api_keys.iter().any(|k| k.key == api_key && !k.revoked) {
                    user_id = Some(uid.clone());
                    break;
                }
            }
        }

        match user_id {
            Some(uid) => self.check_permission(&uid, instance_id, resource_type, action).await,
            None => {
                self.audit_logger.log_invalid_api_key(api_key).await;
                Ok(false)
            }
        }
    }

    /// 获取用户权限
    async fn get_user_permissions(&self, user: &User) -> Result<Vec<PermissionRule>, Box<dyn std::error::Error>> {
        // 检查缓存
        if let Some(cached_rules) = self.permission_cache.read().await.get(&user.id) {
            return Ok(cached_rules.clone());
        }

        let mut all_rules = Vec::new();

        // 获取用户直接拥有的角色的权限
        for role_name in &user.roles {
            if let Some(role) = self.roles.read().await.get(role_name) {
                all_rules.extend_from_slice(&role.permissions);

                // 获取继承的角色权限
                for inherited_role_name in &role.inherit_from {
                    if let Some(inherited_role) = self.roles.read().await.get(inherited_role_name) {
                        all_rules.extend_from_slice(&inherited_role.permissions);
                    }
                }
            }
        }

        // 缓存权限
        {
            let mut cache = self.permission_cache.write().await;
            cache.insert(user.id.clone(), all_rules.clone());
        }

        Ok(all_rules)
    }

    /// 检查实例是否被允许
    fn is_instance_allowed(&self, rule: &PermissionRule, instance_id: &str) -> bool {
        // 如果有明确拒绝的实例，则不允许
        if !rule.denied_instances.is_empty() && rule.denied_instances.contains(instance_id) {
            return false;
        }

        // 如果允许的实例列表为空，则允许所有实例
        if rule.allowed_instances.is_empty() {
            return true;
        }

        // 否则只允许列表中的实例
        rule.allowed_instances.contains(instance_id)
    }

    /// 检查规则是否有效
    fn is_rule_valid(&self, rule: &PermissionRule) -> bool {
        let now = Utc::now();
        
        if let Some(valid_from) = rule.valid_from {
            if now < valid_from {
                return false;
            }
        }

        if let Some(valid_until) = rule.valid_until {
            if now > valid_until {
                return false;
            }
        }

        true
    }

    /// 评估条件
    async fn evaluate_condition(&self, rule: &PermissionRule, user_id: &str, instance_id: &str) -> bool {
        match rule.condition.as_deref() {
            Some("is_owner") => {
                // 检查用户是否是实例的所有者
                // 这里需要查询实例所有权信息
                true  // 简化实现
            }
            Some("is_team_member") => {
                // 检查用户是否属于相关团队
                // 这里需要查询团队成员信息
                true  // 简化实现
            }
            Some("is_assigned_task") => {
                // 检查用户是否被分配了相关任务
                // 这里需要查询任务分配信息
                true  // 简化实现
            }
            None => true,
            _ => false,
        }
    }

    /// 添加用户
    pub async fn add_user(&self, user: User) -> Result<(), Box<dyn std::error::Error>> {
        let mut users = self.users.write().await;
        users.insert(user.id.clone(), user.clone());
        
        // 清除相关缓存
        let mut cache = self.permission_cache.write().await;
        cache.remove(&user.id);
        
        Ok(())
    }

    /// 添加角色
    pub async fn add_role(&self, role: Role) -> Result<(), Box<dyn std::error::Error>> {
        let mut roles = self.roles.write().await;
        roles.insert(role.id.clone(), role);
        
        // 清除所有用户的缓存（因为角色定义改变了）
        self.permission_cache.write().await.clear();
        
        Ok(())
    }

    /// 为用户分配角色
    pub async fn assign_role_to_user(&self, user_id: &str, role_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut users = self.users.write().await;
        if let Some(mut user) = users.get_mut(user_id) {
            if !user.roles.contains(&role_name.to_string()) {
                user.roles.push(role_name.to_string());
                user.updated_at = Utc::now();
                
                // 清除该用户的权限缓存
                self.permission_cache.write().await.remove(user_id);
            }
        } else {
            return Err("用户不存在".into());
        }
        
        Ok(())
    }

    /// 验证和刷新权限缓存
    pub async fn refresh_permission_cache(&self) -> Result<(), Box<dyn std::error::Error>> {
        let users = self.users.read().await;
        let mut cache = self.permission_cache.write().await;
        
        // 清空现有缓存
        cache.clear();
        
        // 为所有用户重新生成权限
        for (user_id, user) in users.iter() {
            let permissions = self.get_user_permissions(user).await?;
            cache.insert(user_id.clone(), permissions);
        }
        
        Ok(())
    }
}

/// 审计日志记录器
pub struct AuditLogger;

impl AuditLogger {
    pub fn new() -> Self {
        Self
    }

    pub async fn log_access_denied(
        &self,
        user_id: &str,
        instance_id: &str,
        resource_type: &ResourceType,
        action: &Action,
    ) {
        println!("[AUDIT] Access denied - User: {}, Instance: {}, Resource: {:?}, Action: {:?}", 
                 user_id, instance_id, resource_type, action);
        // 在实际实现中，这里会写入审计日志数据库或文件
    }

    pub async fn log_invalid_api_key(&self, api_key: &str) {
        println!("[AUDIT] Invalid API key used: {}", mask_api_key(api_key));
        // 在实际实现中，这里会记录到安全日志
    }

    pub async fn log_successful_access(
        &self,
        user_id: &str,
        instance_id: &str,
        resource_type: &ResourceType,
        action: &Action,
    ) {
        println!("[AUDIT] Access granted - User: {}, Instance: {}, Resource: {:?}, Action: {:?}", 
                 user_id, instance_id, resource_type, action);
        // 在实际实现中，这里会写入审计日志数据库或文件
    }
}

/// 隐藏 API 密钥的一部分字符
fn mask_api_key(key: &str) -> String {
    if key.len() > 8 {
        let (first, last) = key.split_at(4);
        let (_, last) = last.split_at(last.len() - 4);
        format!("{}...{}", first, last)
    } else {
        "********".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_access_control() {
        let acm = AccessControlManager::new();
        
        // 创建测试用户
        let user = User {
            id: "test_user".to_string(),
            username: "test_user".to_string(),
            email: "test@example.com".to_string(),
            roles: vec!["ceo".to_string()],
            api_keys: vec![ApiKey {
                key: "sk-test-key-1234567890".to_string(),
                name: "Test Key".to_string(),
                scopes: vec!["read".to_string(), "write".to_string()],
                created_at: Utc::now(),
                expires_at: None,
                revoked: false,
            }],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login: None,
        };
        
        acm.add_user(user).await.unwrap();
        
        // 测试权限检查
        let allowed = acm.check_permission(
            "test_user",
            "instance1",
            &ResourceType::Instance,
            &Action::Read,
        ).await.unwrap();
        
        assert!(allowed);
        
        // 测试 API 密钥权限检查
        let api_allowed = acm.check_api_key_permission(
            "sk-test-key-1234567890",
            "instance1",
            &ResourceType::Api,
            &Action::Execute,
        ).await.unwrap();
        
        assert!(api_allowed);
    }
}