//! Skills 编排系统（简化版）
//! 用于管理和调度 MultiClaw 中的各种技能和工具

use crate::a2a::{A2AMessage, A2AGateway};
use crate::core::{MemoryCore, ResourceCore, HealthCore};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::fmt::Debug;

// 导入 ResourceUsage 以避免错误
use crate::core::resource_core::ResourceUsage;

/// 技能类型枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillType {
    /// CEO 技能 - 实例管理
    CEO,
    /// 团队负责人技能 - 团队协调
    TeamLead,
    /// 工作技能 - 具体执行任务
    Worker,
    /// 工具技能 - 系统工具
    Tool,
    /// 自定义技能
    Custom(String),
}

/// 技能执行上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillContext {
    /// 技能 ID
    pub skill_id: String,
    /// 执行者 ID
    pub executor_id: String,
    /// 执行者类型
    pub executor_type: ExecutorType,
    /// 输入参数
    pub inputs: HashMap<String, serde_json::Value>,
    /// 访问令牌
    pub access_token: String,
    /// 执行优先级
    pub priority: u8,
    /// 超时时间（秒）
    pub timeout_secs: u64,
    /// 执行开始时间
    pub start_time: DateTime<Utc>,
}

/// 执行者类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ExecutorType {
    /// 董事长
    Chairman,
    /// CEO
    CEO,
    /// 团队负责人
    TeamLead,
    /// 工作 Agent
    Worker,
    /// 系统
    System,
}

/// 技能元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    /// 技能名称
    pub name: String,
    /// 技能描述
    pub description: String,
    /// 技能类型
    pub skill_type: SkillType,
    /// 版本号
    pub version: String,
    /// 执行者类型要求
    pub required_executor_type: ExecutorType,
    /// 输入参数定义
    pub input_schema: serde_json::Value,
    /// 输出参数定义
    pub output_schema: serde_json::Value,
    /// 执行所需资源
    pub resource_requirements: ResourceRequirements,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
    /// 作者
    pub author: String,
    /// 标签
    pub tags: Vec<String>,
    /// 分类
    pub category: String,
}

/// 资源需求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// 计算资源需求
    pub compute: Option<u64>,
    /// 内存资源需求 (MB)
    pub memory: Option<u64>,
    /// 存储资源需求 (MB)
    pub storage: Option<u64>,
    /// 网络带宽需求 (kbps)
    pub bandwidth: Option<u64>,
    /// API 调用次数需求
    pub api_calls: Option<u64>,
    /// Token 使用量需求
    pub tokens: Option<u64>,
    /// 并发 Agent 需求
    pub concurrent_agents: Option<u64>,
}

/// 技能执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExecutionResult {
    /// 执行 ID
    pub execution_id: String,
    /// 技能 ID
    pub skill_id: String,
    /// 执行状态
    pub status: ExecutionStatus,
    /// 执行结果
    pub result: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
    /// 执行耗时 (毫秒)
    pub execution_time_ms: u128,
    /// 使用的资源
    pub resources_used: ResourceUsage,
    /// 完成时间
    pub completed_at: DateTime<Utc>,
}

/// 执行状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// 等待中
    Pending,
    /// 执行中
    Running,
    /// 成功
    Success,
    /// 失败
    Failed,
    /// 超时
    Timeout,
    /// 取消
    Cancelled,
}

/// 技能执行计划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExecutionPlan {
    /// 计划 ID
    pub plan_id: String,
    /// 目标技能
    pub target_skills: Vec<SkillReference>,
    /// 执行顺序
    pub execution_order: ExecutionOrder,
    /// 依赖关系
    pub dependencies: HashMap<String, Vec<String>>,
    /// 执行上下文
    pub context: SkillContext,
    /// 资源分配
    pub resource_allocation: HashMap<String, ResourceRequirements>,
    /// 预期输出
    pub expected_outputs: Vec<String>,
}

/// 技能引用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillReference {
    /// 技能 ID
    pub id: String,
    /// 参数映射
    pub parameter_mapping: HashMap<String, String>,
    /// 执行条件
    pub execution_condition: Option<String>,
}

/// 执行顺序类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionOrder {
    /// 顺序执行
    Sequential,
    /// 并行执行
    Parallel,
    /// 自定义依赖图
    DependencyGraph,
}

/// 技能执行器 trait
#[async_trait::async_trait]
pub trait SkillExecutor: Send + Sync {
    /// 执行技能
    async fn execute(&self, context: SkillContext) -> Result<SkillExecutionResult, Box<dyn std::error::Error>>;
    /// 获取技能元数据
    fn metadata(&self) -> &SkillMetadata;
    /// 获取执行器名称
    fn name(&self) -> &str;
}

/// Skills 编排系统核心
pub struct SkillsOrchestration {
    /// 注册的技能
    skills: DashMap<String, Arc<dyn SkillExecutor>>,
    /// 技能元数据
    skill_metadata: DashMap<String, SkillMetadata>,
    /// 正在执行的任务
    active_executions: DashMap<String, SkillExecutionResult>,
    /// 技能执行计划
    execution_plans: DashMap<String, SkillExecutionPlan>,
    /// A2A 网关引用
    a2a_gateway: Arc<A2AGateway>,
    /// 记忆核心引用
    memory_core: Arc<MemoryCore>,
    /// 资源核心引用
    resource_core: Arc<ResourceCore>,
    /// 健康核心引用
    health_core: Arc<HealthCore>,
    /// 技能队列
    skill_queue: Arc<RwLock<Vec<SkillContext>>>,
    /// 执行历史
    execution_history: DashMap<String, SkillExecutionResult>,
    /// 最大历史记录数
    max_history_size: usize,
}

impl SkillsOrchestration {
    /// 创建新的 Skills 编排系统实例
    pub fn new(
        a2a_gateway: Arc<A2AGateway>,
        memory_core: Arc<MemoryCore>,
        resource_core: Arc<ResourceCore>,
        health_core: Arc<HealthCore>,
    ) -> Self {
        Self {
            skills: DashMap::new(),
            skill_metadata: DashMap::new(),
            active_executions: DashMap::new(),
            execution_plans: DashMap::new(),
            a2a_gateway,
            memory_core,
            resource_core,
            health_core,
            skill_queue: Arc::new(RwLock::new(Vec::new())),
            execution_history: DashMap::new(),
            max_history_size: 1000,
        }
    }

    /// 注册技能
    pub fn register_skill(&self, executor: Arc<dyn SkillExecutor>) {
        let metadata = executor.metadata().clone();
        let skill_id = format!("{}_{}", metadata.name, Uuid::new_v4().to_string());
        
        self.skills.insert(skill_id.clone(), executor);
        self.skill_metadata.insert(skill_id, metadata);
    }

    /// 查找技能
    pub fn find_skill(&self, skill_name: &str) -> Option<Arc<dyn SkillExecutor>> {
        for entry in self.skills.iter() {
            let executor = entry.value();
            if executor.name() == skill_name {
                return Some(Arc::clone(executor));
            }
        }
        None
    }

    /// 执行单个技能
    pub async fn execute_skill(&self, context: SkillContext) -> Result<SkillExecutionResult, Box<dyn std::error::Error>> {
        let skill_executor = self.find_skill(&context.skill_id);
        
        if let Some(executor) = skill_executor {
            // 检查资源需求
            let metadata = executor.metadata();
            if let Some(requirements) = self.check_resource_requirements(&metadata.resource_requirements).await {
                return Ok(SkillExecutionResult {
                    execution_id: Uuid::new_v4().to_string(),
                    skill_id: context.skill_id,
                    status: ExecutionStatus::Failed,
                    result: None,
                    error: Some(format!("Insufficient resources: {}", requirements)),
                    execution_time_ms: 0,
                    resources_used: ResourceUsage::default(),
                    completed_at: Utc::now(),
                });
            }

            // 添加到执行队列
            {
                let mut queue = self.skill_queue.write().await;
                queue.push(context.clone());
            }

            // 执行技能 - 使用克隆的上下文以避免所有权问题
            let start_time = std::time::Instant::now();
            let result = executor.execute(context.clone()).await;
            let execution_time = start_time.elapsed().as_millis();

            match result {
                Ok(execution_result) => {
                    let execution_id = Uuid::new_v4().to_string();
                    
                    let final_result = SkillExecutionResult {
                        execution_id,
                        execution_time_ms: execution_time,
                        completed_at: Utc::now(),
                        ..execution_result
                    };

                    // 记录到历史
                    self.record_execution_history(final_result.clone()).await;

                    Ok(final_result)
                }
                Err(e) => {
                    let execution_id = Uuid::new_v4().to_string();
                    
                    let error_result = SkillExecutionResult {
                        execution_id,
                        skill_id: context.skill_id.clone(),
                        status: ExecutionStatus::Failed,
                        result: None,
                        error: Some(e.to_string()),
                        execution_time_ms: execution_time,
                        resources_used: ResourceUsage::default(),
                        completed_at: Utc::now(),
                    };

                    // 记录到历史
                    self.record_execution_history(error_result.clone()).await;

                    Ok(error_result)
                }
            }
        } else {
            Err(format!("Skill '{}' not found", context.skill_id).into())
        }
    }

    /// 检查资源需求是否满足
    async fn check_resource_requirements(&self, requirements: &ResourceRequirements) -> Option<String> {
        // 这里简化处理，实际实现中需要检查各项资源是否满足需求
        if let Some(memory_req) = requirements.memory {
            // 检查内存资源
            if memory_req > 1024 { // 假设最大可用内存为 1GB
                return Some(format!("Insufficient memory: requested {}MB, available 1024MB", memory_req));
            }
        }

        if let Some(compute_req) = requirements.compute {
            // 检查计算资源
            if compute_req > 100 { // 假设计算单位限制
                return Some(format!("Insufficient compute: requested {}, available 100 units", compute_req));
            }
        }

        None
    }

    /// 记录执行历史
    async fn record_execution_history(&self, result: SkillExecutionResult) {
        let execution_id = result.execution_id.clone();
        self.execution_history.insert(execution_id, result);

        // 限制历史记录大小
        if self.execution_history.len() > self.max_history_size {
            // 简化处理：清除最旧的记录
            let mut oldest_keys: Vec<String> = self.execution_history
                .iter()
                .map(|entry| entry.key().clone())
                .collect();
            
            // 按时间排序，找出最旧的记录
            oldest_keys.sort_by(|a, b| {
                if let (Some(result_a), Some(result_b)) = (self.execution_history.get(a), self.execution_history.get(b)) {
                    result_a.completed_at.cmp(&result_b.completed_at)
                } else {
                    std::cmp::Ordering::Equal
                }
            });
            
            let keys_to_remove = oldest_keys
                .into_iter()
                .take(self.execution_history.len() - self.max_history_size + 100) // 保留一些缓冲
                .collect::<Vec<_>>();
            
            for key in keys_to_remove {
                self.execution_history.remove(&key);
            }
        }
    }

    /// 创建执行计划
    pub async fn create_execution_plan(&self, plan: SkillExecutionPlan) -> Result<String, Box<dyn std::error::Error>> {
        let plan_id = Uuid::new_v4().to_string();
        let mut plan = plan;
        plan.plan_id = plan_id.clone();
        
        self.execution_plans.insert(plan_id.clone(), plan);
        
        Ok(plan_id)
    }

    /// 执行执行计划
    pub async fn execute_plan(&self, plan_id: &str) -> Result<Vec<SkillExecutionResult>, Box<dyn std::error::Error>> {
        if let Some(plan) = self.execution_plans.get(plan_id) {
            let mut results = Vec::new();
            
            match &plan.execution_order {
                ExecutionOrder::Sequential => {
                    for skill_ref in &plan.target_skills {
                        // 创建上下文
                        let mut context = plan.context.clone();
                        context.skill_id = skill_ref.id.clone();
                        
                        if let Ok(result) = self.execute_skill(context).await {
                            results.push(result);
                        }
                    }
                }
                ExecutionOrder::Parallel => {
                    // 并行执行技能
                    // 这里需要使用 tokio::join! 或其他并行执行机制
                    for skill_ref in &plan.target_skills {
                        let mut context = plan.context.clone();
                        context.skill_id = skill_ref.id.clone();
                        
                        if let Ok(result) = self.execute_skill(context).await {
                            results.push(result);
                        }
                    }
                }
                ExecutionOrder::DependencyGraph => {
                    // 根据依赖关系图执行
                    // 这里需要实现拓扑排序算法
                    // 简化实现：顺序执行
                    for skill_ref in &plan.target_skills {
                        let mut context = plan.context.clone();
                        context.skill_id = skill_ref.id.clone();
                        
                        if let Ok(result) = self.execute_skill(context).await {
                            results.push(result);
                        }
                    }
                }
            }
            
            Ok(results)
        } else {
            Err(format!("Execution plan '{}' not found", plan_id).into())
        }
    }

    /// 获取执行历史
    pub async fn get_execution_history(&self, limit: Option<usize>, offset: Option<usize>) -> Vec<SkillExecutionResult> {
        let mut history: Vec<SkillExecutionResult> = self.execution_history
            .iter()
            .map(|entry| entry.value().clone())
            .collect();

        // 按时间排序（最新的在前）
        history.sort_by(|a, b| b.completed_at.cmp(&a.completed_at));

        // 应用分页
        if let Some(off) = offset {
            if off < history.len() {
                history.drain(0..off);
            } else {
                history.clear();
            }
        }

        if let Some(lim) = limit {
            if history.len() > lim {
                history.truncate(lim);
            }
        }

        history
    }

    /// 获取技能统计信息
    pub async fn get_skill_statistics(&self) -> SkillStatistics {
        let mut stats = SkillStatistics::default();
        
        for entry in self.execution_history.iter() {
            let result = entry.value();
            match result.status {
                ExecutionStatus::Success => stats.successful_executions += 1,
                ExecutionStatus::Failed => stats.failed_executions += 1,
                ExecutionStatus::Timeout => stats.timeout_executions += 1,
                ExecutionStatus::Cancelled => stats.cancelled_executions += 1,
                _ => stats.pending_executions += 1,
            }
            
            stats.total_executions += 1;
        }

        stats
    }
    

    /// 获取当前活跃的执行
    pub async fn get_active_executions(&self) -> Vec<SkillExecutionResult> {
        self.active_executions
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// 取消执行
    pub async fn cancel_execution(&self, execution_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        // 在实际实现中，这里需要中断正在执行的任务
        // 简化实现：更新状态为取消
        if let Some(mut result) = self.execution_history.get_mut(execution_id) {
            result.value_mut().status = ExecutionStatus::Cancelled;
            Ok(())
        } else {
            Err(format!("Execution '{}' not found", execution_id).into())
        }
    }
}

/// 技能统计信息
#[derive(Debug, Clone, Default)]
pub struct SkillStatistics {
    pub total_executions: usize,
    pub successful_executions: usize,
    pub failed_executions: usize,
    pub timeout_executions: usize,
    pub cancelled_executions: usize,
    pub pending_executions: usize,
}

// 内置技能实现示例

/// 示例技能：信息收集技能
pub struct InformationGatheringSkill {
    metadata: SkillMetadata,
}

impl InformationGatheringSkill {
    pub fn new() -> Self {
        Self {
            metadata: SkillMetadata {
                name: "information_gathering".to_string(),
                description: "收集和整理相关信息的技能".to_string(),
                skill_type: SkillType::Worker,
                version: "1.0.0".to_string(),
                required_executor_type: ExecutorType::Worker,
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "搜索查询"
                        },
                        "sources": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "搜索来源"
                        }
                    },
                    "required": ["query"]
                }),
                output_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "results": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "title": { "type": "string" },
                                    "content": { "type": "string" },
                                    "source": { "type": "string" },
                                    "relevance_score": { "type": "number" }
                                }
                            }
                        }
                    }
                }),
                resource_requirements: ResourceRequirements {
                    compute: Some(10),
                    memory: Some(64),
                    storage: Some(10),
                    bandwidth: Some(100),
                    api_calls: Some(5),
                    tokens: Some(1000),
                    concurrent_agents: Some(1),
                },
                created_at: Utc::now(),
                updated_at: Utc::now(),
                author: "MultiClaw System".to_string(),
                tags: vec!["research".to_string(), "information".to_string()],
                category: "Data Collection".to_string(),
            },
        }
    }
}

#[async_trait::async_trait]
impl SkillExecutor for InformationGatheringSkill {
    async fn execute(&self, context: SkillContext) -> Result<SkillExecutionResult, Box<dyn std::error::Error>> {
        // 模拟信息收集过程
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        let result = serde_json::json!({
            "results": [
                {
                    "title": "Sample Result 1",
                    "content": "This is sample content for demonstration purposes.",
                    "source": "Demo Source",
                    "relevance_score": 0.95
                }
            ]
        });

        Ok(SkillExecutionResult {
            execution_id: Uuid::new_v4().to_string(),
            skill_id: self.metadata.name.clone(),
            status: ExecutionStatus::Success,
            result: Some(result),
            error: None,
            execution_time_ms: 100,
            resources_used: ResourceUsage::default(),
            completed_at: Utc::now(),
        })
    }

    fn metadata(&self) -> &SkillMetadata {
        &self.metadata
    }

    fn name(&self) -> &str {
        &self.metadata.name
    }
}

/// 示例技能：数据分析技能
pub struct DataAnalysisSkill {
    metadata: SkillMetadata,
}

impl DataAnalysisSkill {
    pub fn new() -> Self {
        Self {
            metadata: SkillMetadata {
                name: "data_analysis".to_string(),
                description: "分析数据并生成洞察的技能".to_string(),
                skill_type: SkillType::Worker,
                version: "1.0.0".to_string(),
                required_executor_type: ExecutorType::Worker,
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "data": {
                            "type": "array",
                            "items": { "type": "object" },
                            "description": "待分析的数据"
                        },
                        "analysis_type": {
                            "type": "string",
                            "enum": ["statistical", "trend", "correlation"],
                            "description": "分析类型"
                        }
                    },
                    "required": ["data", "analysis_type"]
                }),
                output_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "insights": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "分析得出的洞察"
                        },
                        "summary": { "type": "string" }
                    }
                }),
                resource_requirements: ResourceRequirements {
                    compute: Some(20),
                    memory: Some(128),
                    storage: Some(20),
                    bandwidth: Some(50),
                    api_calls: Some(2),
                    tokens: Some(2000),
                    concurrent_agents: Some(1),
                },
                created_at: Utc::now(),
                updated_at: Utc::now(),
                author: "MultiClaw System".to_string(),
                tags: vec!["analysis".to_string(), "data".to_string()],
                category: "Data Processing".to_string(),
            },
        }
    }
}

#[async_trait::async_trait]
impl SkillExecutor for DataAnalysisSkill {
    async fn execute(&self, context: SkillContext) -> Result<SkillExecutionResult, Box<dyn std::error::Error>> {
        // 模拟数据分析过程
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        
        let result = serde_json::json!({
            "insights": [
                "Data shows increasing trend in the first quarter",
                "Correlation found between variables X and Y"
            ],
            "summary": "Analysis completed successfully with meaningful insights",
            "input_data_points": match context.inputs.get("data") {
                Some(serde_json::Value::Array(arr)) => arr.len(),
                _ => 0,
            }
        });

        Ok(SkillExecutionResult {
            execution_id: Uuid::new_v4().to_string(),
            skill_id: self.metadata.name.clone(),
            status: ExecutionStatus::Success,
            result: Some(result),
            error: None,
            execution_time_ms: 200,
            resources_used: ResourceUsage::default(),
            completed_at: Utc::now(),
        })
    }

    fn metadata(&self) -> &SkillMetadata {
        &self.metadata
    }

    fn name(&self) -> &str {
        &self.metadata.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_skills_orchestration_basic() {
        // 这里可以添加测试代码
        let a2a_gateway = Arc::new(A2AGateway::new());
        let memory_core = Arc::new(MemoryCore::new(a2a_gateway.clone()));
        let resource_core = Arc::new(ResourceCore::new());
        let health_core = Arc::new(HealthCore::new());

        let skills_orchestration = SkillsOrchestration::new(
            a2a_gateway,
            memory_core,
            resource_core,
            health_core,
        );

        // 注册示例技能
        skills_orchestration.register_skill(Arc::new(InformationGatheringSkill::new()));
        skills_orchestration.register_skill(Arc::new(DataAnalysisSkill::new()));

        // 验证技能已注册
        assert!(skills_orchestration.find_skill("information_gathering").is_some());
        assert!(skills_orchestration.find_skill("data_analysis").is_some());
    }
}