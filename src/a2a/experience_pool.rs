//! 经验池 - 用于存储和共享 Agent 间的经验
use crate::a2a::experience::{ExperienceCapsule, ExperienceExtractor};
use crate::core::MemoryLevel;
use crate::memory::traits::Memory;
use anyhow::Result;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// 经验存储接口
#[async_trait::async_trait]
pub trait ExperienceStore: Send + Sync {
    /// 存储经验
    async fn store(&self, capsule: &ExperienceCapsule) -> Result<()>;

    /// 检索相关经验
    async fn search(&self, query: &ExperienceQuery) -> Result<Vec<ExperienceCapsule>>;

    /// 更新使用统计
    async fn update_stats(&self, id: &str, success: bool) -> Result<()>;

    /// 获取高置信度经验
    async fn get_top_experiences(&self, task_type: &str, limit: usize) -> Result<Vec<ExperienceCapsule>>;

    /// 删除经验
    async fn delete(&self, id: &str) -> Result<()>;
}

/// 经验查询参数
#[derive(Debug, Clone)]
pub struct ExperienceQuery {
    pub task_type: Option<String>,
    pub min_confidence: f32,
    pub limit: usize,
    pub tags: Option<Vec<String>>,
}

/// 基于内存的经验存储实现
pub struct InMemoryExperienceStore {
    experiences: DashMap<String, ExperienceCapsule>,
}

impl InMemoryExperienceStore {
    pub fn new() -> Self {
        Self {
            experiences: DashMap::new(),
        }
    }
}

#[async_trait::async_trait]
impl ExperienceStore for InMemoryExperienceStore {
    async fn store(&self, capsule: &ExperienceCapsule) -> Result<()> {
        self.experiences.insert(capsule.id.clone(), capsule.clone());
        Ok(())
    }

    async fn search(&self, query: &ExperienceQuery) -> Result<Vec<ExperienceCapsule>> {
        let mut results: Vec<ExperienceCapsule> = self
            .experiences
            .iter()
            .filter(|entry| {
                let capsule = entry.value();
                
                // 检查任务类型
                if let Some(ref task_type) = query.task_type {
                    if !capsule.task_type.contains(task_type) {
                        return false;
                    }
                }
                
                // 检查置信度阈值
                if capsule.confidence < query.min_confidence {
                    return false;
                }
                
                // 检查标签（如果提供了）
                if let Some(ref tags) = query.tags {
                    // 这里需要扩展 ExperienceCapsule 以包含标签
                    // 为了简单起见，暂时不过滤标签
                }
                
                true
            })
            .map(|entry| entry.value().clone())
            .collect();

        // 按置信度降序排序
        results.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        // 限制返回数量
        results.truncate(query.limit);

        Ok(results)
    }

    async fn update_stats(&self, id: &str, success: bool) -> Result<()> {
        if let Some(mut capsule) = self.experiences.get_mut(id) {
            capsule.usage_count += 1;
            if success {
                capsule.success_count += 1;
            }
            capsule.last_used_at = chrono::Utc::now().timestamp();
        }
        Ok(())
    }

    async fn get_top_experiences(&self, task_type: &str, limit: usize) -> Result<Vec<ExperienceCapsule>> {
        let mut results: Vec<ExperienceCapsule> = self
            .experiences
            .iter()
            .filter(|entry| entry.value().task_type == task_type)
            .map(|entry| entry.value().clone())
            .collect();

        // 按置信度和成功比率排序
        results.sort_by(|a, b| {
            let a_success_rate = if a.usage_count > 0 {
                a.success_count as f32 / a.usage_count as f32
            } else {
                0.0
            };
            let b_success_rate = if b.usage_count > 0 {
                b.success_count as f32 / b.usage_count as f32
            } else {
                0.0
            };

            // 首先按成功比率排序，然后按置信度排序
            match b_success_rate.partial_cmp(&a_success_rate) {
                Some(std::cmp::Ordering::Equal) => b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal),
                other => other.unwrap_or(std::cmp::Ordering::Equal),
            }
        });

        results.truncate(limit);
        Ok(results)
    }

    async fn delete(&self, id: &str) -> Result<()> {
        self.experiences.remove(id);
        Ok(())
    }
}

/// 经验池 - 团队/集群级别共享
pub struct ExperiencePool {
    /// 存储后端
    store: Arc<dyn ExperienceStore>,
    /// 订阅者 (agent_id -> task_types)
    subscribers: DashMap<String, Vec<String>>,
    /// 所属级别
    level: MemoryLevel,
    /// 所属团队/实例
    scope_id: String,
    /// 经验提取器 (用于提炼新经验)
    extractor: Arc<ExperienceExtractor>,
}

impl ExperiencePool {
    /// 创建新的经验池
    pub fn new(
        store: Arc<dyn ExperienceStore>,
        level: MemoryLevel,
        scope_id: String,
        extractor: Arc<ExperienceExtractor>,
    ) -> Self {
        Self {
            store,
            subscribers: DashMap::new(),
            level,
            scope_id,
            extractor,
        }
    }

    /// 发布经验到池中
    pub async fn publish(&self, capsule: ExperienceCapsule) -> Result<()> {
        // 1. 存储到本地
        self.store.store(&capsule).await?;

        // 2. 通知订阅者
        self.notify_subscribers(&capsule).await?;

        // 3. 如果是团队级别，传播到集群级别
        if self.level == MemoryLevel::Team {
            self.propagate_to_cluster(&capsule).await?;
        }

        Ok(())
    }

    /// 订阅相关经验
    pub async fn subscribe(&self, agent_id: &str, task_types: &[String]) -> Result<()> {
        self.subscribers
            .insert(agent_id.to_string(), task_types.to_vec());
        Ok(())
    }

    /// 获取相关经验
    pub async fn get_relevant(&self, task_type: &str, context: &str) -> Result<Vec<ExperienceCapsule>> {
        // 1. 按任务类型搜索
        let mut results = self.store.search(&ExperienceQuery {
            task_type: Some(task_type.to_string()),
            min_confidence: 0.6,
            limit: 10,
            tags: None,
        }).await?;

        // 2. 按上下文相似度排序
        results.sort_by(|a, b| {
            let sim_a = self.context_similarity(context, &a.trigger_conditions);
            let sim_b = self.context_similarity(context, &b.trigger_conditions);
            sim_b.partial_cmp(&sim_a).unwrap()
        });

        Ok(results)
    }

    /// 从执行轨迹创建经验并发布
    pub async fn create_and_publish_from_trace(&self, trace: &crate::a2a::experience::ExecutionTrace) -> Result<()> {
        if let Some(capsule) = self.extractor.extract(trace).await? {
            self.publish(capsule).await?;
        }
        Ok(())
    }

    /// 更新经验使用统计
    pub async fn update_experience_stats(&self, id: &str, success: bool) -> Result<()> {
        self.store.update_stats(id, success).await
    }

    /// 获取热门经验
    pub async fn get_top_experiences(&self, task_type: &str, limit: usize) -> Result<Vec<ExperienceCapsule>> {
        self.store.get_top_experiences(task_type, limit).await
    }

    /// 计算上下文相似度
    fn context_similarity(&self, context: &str, conditions: &[crate::a2a::experience::Condition]) -> f32 {
        let mut similarity = 0.0;
        let total_weight: f32 = conditions.iter().map(|c| c.weight).sum();
        
        if total_weight == 0.0 {
            return 0.0;
        }
        
        for condition in conditions {
            // 简单的文本包含检查
            if context.to_lowercase().contains(&condition.parameter.to_lowercase()) {
                similarity += condition.weight;
            }
        }
        
        similarity / total_weight
    }

    /// 通知订阅者新经验
    async fn notify_subscribers(&self, capsule: &ExperienceCapsule) -> Result<()> {
        // 找到对此任务类型感兴趣的订阅者
        for subscriber in self.subscribers.iter() {
            let agent_id = subscriber.key();
            let task_types = subscriber.value();
            
            if task_types.contains(&capsule.task_type) {
                // 在实际实现中，这里会通过某种方式通知订阅者
                // 例如通过 A2A 消息或其他通信机制
                println!("Notifying agent {} of new experience for task type {}", agent_id, capsule.task_type);
            }
        }
        Ok(())
    }

    /// 传播到集群级别
    async fn propagate_to_cluster(&self, capsule: &ExperienceCapsule) -> Result<()> {
        // 在实际实现中，这里会将经验传播到集群级别的经验池
        // 可能通过 A2A 通信或其他分布式机制
        println!("Propagating experience {} to cluster level", capsule.id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_experience_store() {
        let store = Arc::new(InMemoryExperienceStore::new());
        
        let capsule = ExperienceCapsule {
            id: "test-id".to_string(),
            source_agent: "test-agent".to_string(),
            task_type: "test-task".to_string(),
            strategy: Default::default(),
            trigger_conditions: vec![],
            expected_outcome: "test-outcome".to_string(),
            actual_outcome: crate::a2a::experience::Outcome::Success,
            confidence: 0.8,
            usage_count: 0,
            success_count: 0,
            created_at: 0,
            last_used_at: 0,
        };

        // 存储经验
        store.store(&capsule).await.unwrap();
        
        // 搜索经验
        let results = store.search(&ExperienceQuery {
            task_type: Some("test-task".to_string()),
            min_confidence: 0.5,
            limit: 10,
            tags: None,
        }).await.unwrap();
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "test-id");
    }

    #[tokio::test]
    async fn test_experience_pool() {
        let store = Arc::new(InMemoryExperienceStore::new());
        let extractor = Arc::new(create_dummy_extractor().await);
        
        let pool = ExperiencePool::new(
            store,
            MemoryLevel::Team,
            "test-scope".to_string(),
            extractor,
        );
        
        let capsule = ExperienceCapsule {
            id: "test-id".to_string(),
            source_agent: "test-agent".to_string(),
            task_type: "test-task".to_string(),
            strategy: Default::default(),
            trigger_conditions: vec![],
            expected_outcome: "test-outcome".to_string(),
            actual_outcome: crate::a2a::experience::Outcome::Success,
            confidence: 0.8,
            usage_count: 0,
            success_count: 0,
            created_at: 0,
            last_used_at: 0,
        };

        // 发布经验
        pool.publish(capsule).await.unwrap();
        
        // 获取相关经验
        let results = pool.get_relevant("test-task", "").await.unwrap();
        assert_eq!(results.len(), 1);
    }

    // 辅助函数：创建虚拟提取器用于测试
    async fn create_dummy_extractor() -> ExperienceExtractor {
        use crate::a2a::gateway::A2AGateway;
        use crate::providers::openai::OpenAiProvider;
        
        // 创建一个虚拟的 provider
        struct DummyProvider;
        #[async_trait::async_trait]
        impl crate::providers::Provider for DummyProvider {
            async fn chat_with_system(
                &self,
                _system_prompt: Option<&str>,
                _message: &str,
                _model: &str,
                _temperature: f64,
            ) -> anyhow::Result<String> {
                Ok("dummy response".to_string())
            }
        }
        
        let provider: Arc<dyn crate::providers::Provider> = Arc::new(DummyProvider);
        let gateway = A2AGateway::new_for_test(); // 假设有这样的测试构造函数
        
        ExperienceExtractor::new(
            provider,
            gateway,
            Default::default(),
        )
    }
}