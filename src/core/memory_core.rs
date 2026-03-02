//! MemoryCore - 分级记忆核心模块
//! 实现全局/集群/团队/本地四级记忆系统

use crate::a2a::{A2AMessage, A2AMessageType, A2AGateway};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// 记忆级别枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryLevel {
    /// 全局级别 - 董事长/全局共享
    Global,
    /// 集群级别 - CEO/实例内共享
    Cluster,
    /// 团队级别 - 团队内共享
    Team,
    /// 本地级别 - 单个 Agent 内部
    Local,
}

/// 记忆条目结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// 记忆唯一 ID
    pub id: String,
    /// 记忆键
    pub key: String,
    /// 记忆内容
    pub content: String,
    /// 记忆级别
    pub level: MemoryLevel,
    /// 所属团队 ID (可选)
    pub team_id: Option<String>,
    /// 所属实例 ID (可选)
    pub instance_id: Option<String>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
    /// 过期时间 (可选)
    pub expires_at: Option<DateTime<Utc>>,
    /// 访问权限
    pub access_permissions: AccessPermissions,
    /// 标签集合
    pub tags: HashSet<String>,
    /// 重要性评分 (0-100)
    pub importance: u8,
}

/// 访问权限结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPermissions {
    /// 读权限 - 哪些角色可以读取
    pub read_roles: HashSet<AccessRole>,
    /// 写权限 - 哪些角色可以写入
    pub write_roles: HashSet<AccessRole>,
    /// 删除权限 - 哪些角色可以删除
    pub delete_roles: HashSet<AccessRole>,
}

/// 访问角色枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccessRole {
    /// 董事长角色
    Chairman,
    /// CEO 角色
    CEO,
    /// 团队负责人角色
    TeamLead,
    /// 普通团队成员角色
    TeamMember,
    /// 工作 Agent 角色
    Worker,
}

/// 记忆查询参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQuery {
    /// 查询关键词
    pub keywords: Vec<String>,
    /// 记忆级别过滤
    pub levels: Option<Vec<MemoryLevel>>,
    /// 团队 ID 过滤
    pub team_ids: Option<Vec<String>>,
    /// 实例 ID 过滤
    pub instance_ids: Option<Vec<String>>,
    /// 标签过滤
    pub tags: Option<Vec<String>>,
    /// 时间范围过滤
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    /// 重要性阈值
    pub min_importance: Option<u8>,
    /// 限制结果数量
    pub limit: Option<usize>,
    /// 偏移量
    pub offset: Option<usize>,
}

/// 记忆搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchResult {
    /// 匹配的记忆条目
    pub entries: Vec<MemoryEntry>,
    /// 总匹配数
    pub total_count: usize,
    /// 查询耗时
    pub query_time_ms: u128,
}

/// 记忆共享策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharingPolicy {
    /// 共享级别
    pub level: MemoryLevel,
    /// 目标角色
    pub target_roles: HashSet<AccessRole>,
    /// 自动过期时间
    pub auto_expire_duration: Option<chrono::Duration>,
    /// 验证回调 (可选)
    pub validation_callback: Option<String>,
}

/// MemoryCore - 分级记忆核心
pub struct MemoryCore {
    /// 内部记忆存储
    memory_store: DashMap<String, MemoryEntry>,
    /// 按级别索引的记忆
    level_index: DashMap<MemoryLevel, HashSet<String>>,
    /// 按团队索引的记忆
    team_index: DashMap<String, HashSet<String>>,
    /// 按实例索引的记忆
    instance_index: DashMap<String, HashSet<String>>,
    /// 按标签索引的记忆
    tag_index: DashMap<String, HashSet<String>>,
    /// A2A 网关引用 (用于跨层级通信)
    a2a_gateway: Arc<A2AGateway>,
    /// 默认共享策略
    default_sharing_policies: HashMap<MemoryLevel, SharingPolicy>,
    /// 记忆验证器
    validators: Vec<Box<dyn MemoryValidator + Send + Sync>>,
}

/// 记忆验证器 trait
pub trait MemoryValidator: Send + Sync {
    /// 验证记忆条目是否有效
    fn validate(&self, entry: &MemoryEntry) -> Result<bool, Box<dyn std::error::Error>>;
    /// 获取验证器名称
    fn name(&self) -> &str;
}

/// 内置记忆验证器实现
pub struct DefaultMemoryValidator;

impl MemoryValidator for DefaultMemoryValidator {
    fn validate(&self, entry: &MemoryEntry) -> Result<bool, Box<dyn std::error::Error>> {
        // 检查是否过期
        if let Some(expires_at) = entry.expires_at {
            if Utc::now() > expires_at {
                return Ok(false);
            }
        }
        
        // 检查重要性是否在合理范围内
        if entry.importance > 100 {
            return Ok(false);
        }
        
        // 检查内容长度
        if entry.content.is_empty() {
            return Ok(false);
        }
        
        Ok(true)
    }

    fn name(&self) -> &str {
        "default_validator"
    }
}

impl MemoryCore {
    /// 创建新的 MemoryCore 实例
    pub fn new(a2a_gateway: Arc<A2AGateway>) -> Self {
        let mut core = Self {
            memory_store: DashMap::new(),
            level_index: DashMap::new(),
            team_index: DashMap::new(),
            instance_index: DashMap::new(),
            tag_index: DashMap::new(),
            a2a_gateway,
            default_sharing_policies: HashMap::new(),
            validators: vec![Box::new(DefaultMemoryValidator)],
        };

        // 设置默认共享策略
        core.setup_default_policies();

        core
    }

    /// 设置默认共享策略
    fn setup_default_policies(&mut self) {
        // 全局级别：董事长和 CEO 可读写
        let mut global_read_roles = HashSet::new();
        global_read_roles.insert(AccessRole::Chairman);
        global_read_roles.insert(AccessRole::CEO);
        
        let mut global_write_roles = HashSet::new();
        global_write_roles.insert(AccessRole::Chairman);
        global_write_roles.insert(AccessRole::CEO);

        let mut global_delete_roles = HashSet::new();
        global_delete_roles.insert(AccessRole::Chairman);

        self.default_sharing_policies.insert(
            MemoryLevel::Global,
            SharingPolicy {
                level: MemoryLevel::Global,
                target_roles: global_read_roles.clone(),
                auto_expire_duration: Some(chrono::Duration::days(365)), // 1年
                validation_callback: None,
            }
        );

        // 集群级别：CEO、团队负责人和团队成员可读，CEO 和团队负责人可写
        let mut cluster_read_roles = HashSet::new();
        cluster_read_roles.insert(AccessRole::CEO);
        cluster_read_roles.insert(AccessRole::TeamLead);
        cluster_read_roles.insert(AccessRole::TeamMember);

        let mut cluster_write_roles = HashSet::new();
        cluster_write_roles.insert(AccessRole::CEO);
        cluster_write_roles.insert(AccessRole::TeamLead);

        let mut cluster_delete_roles = HashSet::new();
        cluster_delete_roles.insert(AccessRole::CEO);
        cluster_delete_roles.insert(AccessRole::TeamLead);

        self.default_sharing_policies.insert(
            MemoryLevel::Cluster,
            SharingPolicy {
                level: MemoryLevel::Cluster,
                target_roles: cluster_read_roles,
                auto_expire_duration: Some(chrono::Duration::days(180)), // 6个月
                validation_callback: None,
            }
        );

        // 团队级别：团队内所有人可读写
        let mut team_read_roles = HashSet::new();
        team_read_roles.insert(AccessRole::TeamLead);
        team_read_roles.insert(AccessRole::TeamMember);
        team_read_roles.insert(AccessRole::Worker);

        let mut team_write_roles = HashSet::new();
        team_write_roles.insert(AccessRole::TeamLead);
        team_write_roles.insert(AccessRole::TeamMember);
        team_write_roles.insert(AccessRole::Worker);

        let mut team_delete_roles = HashSet::new();
        team_delete_roles.insert(AccessRole::TeamLead);
        team_delete_roles.insert(AccessRole::TeamMember);

        self.default_sharing_policies.insert(
            MemoryLevel::Team,
            SharingPolicy {
                level: MemoryLevel::Team,
                target_roles: team_read_roles,
                auto_expire_duration: Some(chrono::Duration::days(90)), // 3个月
                validation_callback: None,
            }
        );

        // 本地级别：只有自己可访问
        let mut local_read_roles = HashSet::new();
        local_read_roles.insert(AccessRole::Worker);

        let mut local_write_roles = HashSet::new();
        local_write_roles.insert(AccessRole::Worker);

        let mut local_delete_roles = HashSet::new();
        local_delete_roles.insert(AccessRole::Worker);

        self.default_sharing_policies.insert(
            MemoryLevel::Local,
            SharingPolicy {
                level: MemoryLevel::Local,
                target_roles: local_read_roles,
                auto_expire_duration: Some(chrono::Duration::days(7)), // 1周
                validation_callback: None,
            }
        );
    }

    /// 存储记忆条目
    pub async fn store_memory(
        &self,
        mut entry: MemoryEntry,
        requesting_role: AccessRole,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // 验证请求角色是否有写权限
        if !self.has_write_permission(&entry, requesting_role) {
            return Err("Insufficient write permissions".into());
        }

        // 验证记忆条目
        for validator in &self.validators {
            if !validator.validate(&entry)? {
                return Err(format!("Memory entry failed validation by {}", validator.name()).into());
            }
        }

        // 如果没有设置 ID，则生成一个
        if entry.id.is_empty() {
            entry.id = Uuid::new_v4().to_string();
        }

        // 如果没有设置创建时间，则使用当前时间
        if entry.created_at.timestamp() == 0 {
            entry.created_at = Utc::now();
        }

        entry.updated_at = Utc::now();

        // 存储记忆
        self.memory_store.insert(entry.id.clone(), entry.clone());

        // 更新索引
        self.update_indexes(&entry);

        // 如果是高层级记忆，可能需要通知其他层级
        self.propagate_memory(&entry).await?;

        Ok(entry.id)
    }

    /// 检查写权限
    fn has_write_permission(&self, entry: &MemoryEntry, role: AccessRole) -> bool {
        entry.access_permissions.write_roles.contains(&role)
    }

    /// 更新各种索引
    fn update_indexes(&self, entry: &MemoryEntry) {
        // 更新级别索引
        {
            let mut level_entries = self.level_index
                .entry(entry.level)
                .or_insert_with(HashSet::new);
            level_entries.insert(entry.id.clone());
        }

        // 更新团队索引
        if let Some(ref team_id) = entry.team_id {
            {
                let mut team_entries = self.team_index
                    .entry(team_id.clone())
                    .or_insert_with(HashSet::new);
                team_entries.insert(entry.id.clone());
            }
        }

        // 更新实例索引
        if let Some(ref instance_id) = entry.instance_id {
            {
                let mut instance_entries = self.instance_index
                    .entry(instance_id.clone())
                    .or_insert_with(HashSet::new);
                instance_entries.insert(entry.id.clone());
            }
        }

        // 更新标签索引
        for tag in &entry.tags {
            {
                let mut tag_entries = self.tag_index
                    .entry(tag.clone())
                    .or_insert_with(HashSet::new);
                tag_entries.insert(entry.id.clone());
            }
        }
    }

    /// 传播记忆到其他层级（如果需要）
    async fn propagate_memory(&self, entry: &MemoryEntry) -> Result<(), Box<dyn std::error::Error>> {
        // 根据共享策略决定是否需要传播记忆
        if let Some(policy) = self.default_sharing_policies.get(&entry.level) {
            // 如果是全局或集群级别的记忆，可能需要传播到其他实例或团队
            match entry.level {
                MemoryLevel::Global | MemoryLevel::Cluster => {
                    // 发送 A2A 消息通知其他相关实例/团队
                    let message = A2AMessage {
                        message_id: Uuid::new_v4().to_string(),
                        sender_id: "memory_core".to_string(),
                        sender_team_id: entry.team_id.clone(),
                        sender_instance_id: entry.instance_id.clone(),
                        recipient_id: "memory_propagation".to_string(), // 这里应该是实际的目标
                        message_type: A2AMessageType::KnowledgeShare {
                            knowledge_type: format!("{:?}", entry.level),
                            content: serde_json::to_string(entry)?,
                            applicable_scenarios: vec!["memory_propagation".to_string()],
                        },
                        content: serde_json::to_value(entry)?,
                        priority: crate::a2a::MessagePriority::Normal,
                        timestamp: Utc::now().timestamp(),
                        related_task_id: None,
                        requires_reply: false,
                        timeout_secs: Some(30),
                    };

                    // 发送消息（这里简化处理）
                    // self.a2a_gateway.send(message).await?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// 检索记忆条目
    pub async fn retrieve_memory(
        &self,
        query: MemoryQuery,
        requesting_role: AccessRole,
    ) -> Result<MemorySearchResult, Box<dyn std::error::Error>> {
        let start_time = std::time::Instant::now();

        // 构建候选记忆 ID 集合
        let mut candidate_ids = HashSet::new();

        // 根据查询条件筛选
        if let Some(levels) = query.levels {
            for level in levels {
                if let Some(level_entries) = self.level_index.get(&level) {
                    for id in level_entries.value() {
                        if let Some(entry) = self.memory_store.get(id) {
                            // 检查读权限
                            if entry.access_permissions.read_roles.contains(&requesting_role) {
                                candidate_ids.insert(id.clone());
                            }
                        }
                    }
                }
            }
        } else {
            // 如果没有指定级别，则检查所有有权限访问的记忆
            for entry_pair in self.memory_store.iter() {
                let entry = entry_pair.value();
                if entry.access_permissions.read_roles.contains(&requesting_role) {
                    candidate_ids.insert(entry.id.clone());
                }
            }
        }

        // 根据团队 ID 过滤
        if let Some(team_ids) = query.team_ids {
            let mut filtered_ids = HashSet::new();
            for team_id in team_ids {
                if let Some(team_entries) = self.team_index.get(&team_id) {
                    for id in team_entries.value() {
                        if candidate_ids.contains(id) {
                            filtered_ids.insert(id.clone());
                        }
                    }
                }
            }
            candidate_ids = filtered_ids;
        }

        // 根据实例 ID 过滤
        if let Some(instance_ids) = query.instance_ids {
            let mut filtered_ids = HashSet::new();
            for instance_id in instance_ids {
                if let Some(instance_entries) = self.instance_index.get(&instance_id) {
                    for id in instance_entries.value() {
                        if candidate_ids.contains(id) {
                            filtered_ids.insert(id.clone());
                        }
                    }
                }
            }
            candidate_ids = filtered_ids;
        }

        // 根据标签过滤
        if let Some(tags) = query.tags {
            let mut filtered_ids = HashSet::new();
            for tag in tags {
                if let Some(tag_entries) = self.tag_index.get(&tag) {
                    for id in tag_entries.value() {
                        if candidate_ids.contains(id) {
                            filtered_ids.insert(id.clone());
                        }
                    }
                }
            }
            candidate_ids = filtered_ids;
        }

        // 应用关键词搜索
        let mut matching_entries = Vec::new();
        for id in candidate_ids {
            if let Some(entry) = self.memory_store.get(&id) {
                let mut matches_keywords = true;
                
                for keyword in &query.keywords {
                    // 搜索 ID、键和内容
                    if !entry.id.contains(keyword) 
                        && !entry.key.contains(keyword) 
                        && !entry.content.contains(keyword) {
                        matches_keywords = false;
                        break;
                    }
                }
                
                if matches_keywords {
                    // 应用时间范围过滤
                    if let Some((start, end)) = query.time_range {
                        if entry.created_at < start || entry.created_at > end {
                            continue;
                        }
                    }
                    
                    // 应用重要性过滤
                    if let Some(min_importance) = query.min_importance {
                        if entry.importance < min_importance {
                            continue;
                        }
                    }
                    
                    matching_entries.push(entry.clone());
                }
            }
        }

        // 排序：按重要性降序，然后按时间倒序
        matching_entries.sort_by(|a, b| {
            b.importance.cmp(&a.importance)
                .then(b.created_at.cmp(&a.created_at))
        });

        // 应用分页
        let total_count = matching_entries.len();
        let mut entries = matching_entries;
        
        if let Some(offset) = query.offset {
            if offset < entries.len() {
                entries.drain(0..offset);
            } else {
                entries.clear();
            }
        }
        
        if let Some(limit) = query.limit {
            if entries.len() > limit {
                entries.truncate(limit);
            }
        }

        let query_time_ms = start_time.elapsed().as_millis();

        Ok(MemorySearchResult {
            entries,
            total_count,
            query_time_ms,
        })
    }

    /// 更新记忆条目
    pub async fn update_memory(
        &self,
        id: &str,
        updates: MemoryUpdates,
        requesting_role: AccessRole,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(mut entry) = self.memory_store.get_mut(id) {
            // 检查写权限
            if !entry.access_permissions.write_roles.contains(&requesting_role) {
                return Err("Insufficient write permissions".into());
            }

            // 应用更新
            let mut entry_ref = entry.value_mut();
            
            if let Some(content) = updates.content {
                entry_ref.content = content;
            }
            
            if let Some(importance) = updates.importance {
                entry_ref.importance = importance;
            }
            
            if let Some(tags) = updates.tags {
                entry_ref.tags = tags;
            }
            
            if let Some(expires_at) = updates.expires_at {
                entry_ref.expires_at = Some(expires_at);
            }
            
            entry_ref.updated_at = Utc::now();

            // 重新构建索引（简单做法是删除旧索引并重建）
            self.rebuild_indexes_for_entry(entry_ref);

            // 传播更新
            self.propagate_memory(entry_ref).await?;
            
            Ok(())
        } else {
            Err("Memory entry not found".into())
        }
    }

    /// 重新构建条目的索引
    fn rebuild_indexes_for_entry(&self, entry: &MemoryEntry) {
        // 从所有索引中移除旧条目
        self.remove_from_indexes(&entry.id, entry);

        // 添加到新索引
        self.update_indexes(entry);
    }

    /// 从索引中移除条目
    fn remove_from_indexes(&self, id: &str, entry: &MemoryEntry) {
        // 从级别索引移除
        if let Some(mut level_entries) = self.level_index.get_mut(&entry.level) {
            level_entries.value_mut().remove(id);
        }

        // 从团队索引移除
        if let Some(ref team_id) = entry.team_id {
            if let Some(mut team_entries) = self.team_index.get_mut(team_id) {
                team_entries.value_mut().remove(id);
            }
        }

        // 从实例索引移除
        if let Some(ref instance_id) = entry.instance_id {
            if let Some(mut instance_entries) = self.instance_index.get_mut(instance_id) {
                instance_entries.value_mut().remove(id);
            }
        }

        // 从标签索引移除
        for tag in &entry.tags {
            if let Some(mut tag_entries) = self.tag_index.get_mut(tag) {
                tag_entries.value_mut().remove(id);
            }
        }
    }

    /// 删除记忆条目
    pub async fn delete_memory(
        &self,
        id: &str,
        requesting_role: AccessRole,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(entry) = self.memory_store.get(id) {
            // 检查删除权限
            if !entry.access_permissions.delete_roles.contains(&requesting_role) {
                return Err("Insufficient delete permissions".into());
            }

            // 移除记忆
            let entry = entry.clone();
            self.memory_store.remove(id);

            // 从索引中移除
            self.remove_from_indexes(&entry.id, &entry);

            Ok(())
        } else {
            Err("Memory entry not found".into())
        }
    }

    /// 获取记忆统计信息
    pub async fn get_statistics(&self) -> MemoryStatistics {
        let mut stats = MemoryStatistics::default();
        
        for entry_pair in self.memory_store.iter() {
            let entry = entry_pair.value();
            match entry.level {
                MemoryLevel::Global => stats.global_count += 1,
                MemoryLevel::Cluster => stats.cluster_count += 1,
                MemoryLevel::Team => stats.team_count += 1,
                MemoryLevel::Local => stats.local_count += 1,
            }
            
            stats.total_count += 1;
            stats.total_size_bytes += entry.content.len();
        }

        stats
    }

    /// 清理过期的记忆
    pub async fn cleanup_expired_memories(&self) -> Result<usize, Box<dyn std::error::Error>> {
        let mut expired_ids = Vec::new();
        
        for entry_pair in self.memory_store.iter() {
            let entry = entry_pair.value();
            if let Some(expires_at) = entry.expires_at {
                if Utc::now() > expires_at {
                    expired_ids.push(entry.id.clone());
                }
            }
        }

        let mut cleaned_count = 0;
        for id in expired_ids {
            if self.memory_store.contains_key(&id) {
                // 使用内部方法删除，避免权限检查
                if let Some(entry) = self.memory_store.remove(&id) {
                    self.remove_from_indexes(&id, &entry.1);
                    cleaned_count += 1;
                }
            }
        }

        Ok(cleaned_count)
    }

    /// 添加自定义验证器
    pub fn add_validator(&mut self, validator: Box<dyn MemoryValidator + Send + Sync>) {
        self.validators.push(validator);
    }
}

/// 记忆更新参数
#[derive(Debug, Clone, Default)]
pub struct MemoryUpdates {
    pub content: Option<String>,
    pub importance: Option<u8>,
    pub tags: Option<HashSet<String>>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// 记忆统计信息
#[derive(Debug, Clone, Default)]
pub struct MemoryStatistics {
    pub total_count: usize,
    pub global_count: usize,
    pub cluster_count: usize,
    pub team_count: usize,
    pub local_count: usize,
    pub total_size_bytes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_memory_core_basic_operations() {
        let a2a_gateway = Arc::new(A2AGateway::new());
        let memory_core = MemoryCore::new(a2a_gateway);

        // 创建一个记忆条目
        let mut tags = HashSet::new();
        tags.insert("test".to_string());
        tags.insert("important".to_string());

        let entry = MemoryEntry {
            id: "".to_string(),
            key: "test_key".to_string(),
            content: "This is a test memory entry".to_string(),
            level: MemoryLevel::Team,
            team_id: Some("team_1".to_string()),
            instance_id: Some("instance_1".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: Some(Utc::now() + chrono::Duration::days(30)),
            access_permissions: AccessPermissions {
                read_roles: [AccessRole::TeamLead, AccessRole::TeamMember].iter().cloned().collect(),
                write_roles: [AccessRole::TeamLead].iter().cloned().collect(),
                delete_roles: [AccessRole::TeamLead].iter().cloned().collect(),
            },
            tags,
            importance: 80,
        };

        // 存储记忆
        let id = memory_core
            .store_memory(entry, AccessRole::TeamLead)
            .await
            .expect("Failed to store memory");

        assert!(!id.is_empty());

        // 检索记忆
        let query = MemoryQuery {
            keywords: vec!["test".to_string()],
            levels: Some(vec![MemoryLevel::Team]),
            team_ids: Some(vec!["team_1".to_string()]),
            instance_ids: Some(vec!["instance_1".to_string()]),
            tags: Some(vec!["test".to_string()]),
            time_range: None,
            min_importance: Some(70),
            limit: Some(10),
            offset: Some(0),
        };

        let result = memory_core
            .retrieve_memory(query, AccessRole::TeamMember)
            .await
            .expect("Failed to retrieve memory");

        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].content, "This is a test memory entry");
        assert_eq!(result.entries[0].importance, 80);

        // 更新记忆
        let updates = MemoryUpdates {
            content: Some("Updated test memory content".to_string()),
            importance: Some(90),
            ..Default::default()
        };

        memory_core
            .update_memory(&id, updates, AccessRole::TeamLead)
            .await
            .expect("Failed to update memory");

        // 验证更新
        let query = MemoryQuery {
            keywords: vec![id.clone()],
            levels: None,
            team_ids: None,
            instance_ids: None,
            tags: None,
            time_range: None,
            min_importance: None,
            limit: Some(1),
            offset: None,
        };

        let result = memory_core
            .retrieve_memory(query, AccessRole::TeamLead)
            .await
            .expect("Failed to retrieve updated memory");

        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].content, "Updated test memory content");
        assert_eq!(result.entries[0].importance, 90);

        // 删除记忆
        memory_core
            .delete_memory(&id, AccessRole::TeamLead)
            .await
            .expect("Failed to delete memory");

        // 验证删除
        let query = MemoryQuery {
            keywords: vec![id],
            levels: None,
            team_ids: None,
            instance_ids: None,
            tags: None,
            time_range: None,
            min_importance: None,
            limit: Some(1),
            offset: None,
        };

        let result = memory_core
            .retrieve_memory(query, AccessRole::TeamLead)
            .await
            .expect("Failed to retrieve after deletion");

        assert_eq!(result.entries.len(), 0);
    }

    #[tokio::test]
    async fn test_memory_core_statistics() {
        let a2a_gateway = Arc::new(A2AGateway::new());
        let memory_core = MemoryCore::new(a2a_gateway);

        // 添加几个不同级别的记忆
        for i in 0..5 {
            let mut tags = HashSet::new();
            tags.insert(format!("tag_{}", i));

            let level = match i % 4 {
                0 => MemoryLevel::Global,
                1 => MemoryLevel::Cluster,
                2 => MemoryLevel::Team,
                _ => MemoryLevel::Local,
            };

            let entry = MemoryEntry {
                id: "".to_string(),
                key: format!("key_{}", i),
                content: format!("content_{}", i),
                level,
                team_id: if level == MemoryLevel::Team || level == MemoryLevel::Local {
                    Some("test_team".to_string())
                } else {
                    None
                },
                instance_id: Some("test_instance".to_string()),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                expires_at: Some(Utc::now() + chrono::Duration::days(30)),
                access_permissions: AccessPermissions {
                    read_roles: [AccessRole::TeamLead].iter().cloned().collect(),
                    write_roles: [AccessRole::TeamLead].iter().cloned().collect(),
                    delete_roles: [AccessRole::TeamLead].iter().cloned().collect(),
                },
                tags,
                importance: 50 + (i as u8) * 10,
            };

            memory_core
                .store_memory(entry, AccessRole::TeamLead)
                .await
                .expect("Failed to store memory");
        }

        let stats = memory_core.get_statistics().await;
        
        assert_eq!(stats.total_count, 5);
        assert!(stats.global_count > 0);
        assert!(stats.cluster_count > 0);
        assert!(stats.team_count > 0);
        assert!(stats.local_count > 0);
    }
}