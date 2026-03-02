//! 记忆压缩器 - 用于将长对话历史压缩为结构化记忆胶囊
use crate::memory::traits::Memory;
use crate::providers::Provider;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// 记忆胶囊 - 压缩后的记忆单元
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCapsule {
    /// 唯一标识
    pub id: String,
    /// 时间范围 [start, end]
    pub time_range: (i64, i64),
    /// 摘要内容
    pub summary: String,
    /// 关键实体提取
    pub entities: Vec<Entity>,
    /// 工具调用记录
    pub tool_calls: Vec<ToolCallSummary>,
    /// 决策点记录
    pub decisions: Vec<Decision>,
    /// 重要性评分 (0.0-1.0)
    pub importance: f32,
    /// 原始对话轮数
    pub original_turns: usize,
    /// 压缩比
    pub compression_ratio: f32,
}

/// 关键实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub name: String,
    pub entity_type: EntityType,
    pub relevance_score: f32,
}

/// 实体类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityType {
    Person,
    Organization,
    Location,
    Concept,
    Topic,
    Task,
    Goal,
    Constraint,
    Resource,
    Custom(String),
}

/// 工具调用摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallSummary {
    pub tool_name: String,
    pub parameters: serde_json::Value,
    pub result: Option<String>,
    pub success: bool,
    pub timestamp: i64,
}

/// 决策点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub question: String,
    pub options: Vec<String>,
    pub chosen_option: String,
    pub rationale: String,
    pub outcome: Option<String>,
    pub timestamp: i64,
}

/// 记忆折叠器配置
#[derive(Debug, Clone, Deserialize)]
pub struct MemoryCompressorConfig {
    /// 触发压缩的上下文阈值 (token 数)
    pub threshold: usize,
    /// 目标压缩比例
    pub target_ratio: f32,
    /// 摘要提示模板
    pub summary_prompt_template: String,
}

impl Default for MemoryCompressorConfig {
    fn default() -> Self {
        Self {
            threshold: 128000, // 128k tokens, 大约 80% 的 16k 模型上下文
            target_ratio: 0.2, // 目标压缩到原来的 20%
            summary_prompt_template: r#"请将以下对话历史压缩为一个结构化的记忆胶囊，包括：
1. 对话的核心主题和要点摘要
2. 重要的实体（人物、地点、概念、资源等）
3. 关键决策点和选择
4. 重要的工具调用和结果

原始对话：
{conversation_history}

请以 JSON 格式返回结果，包含 summary, entities, decisions, tool_calls 字段。"#.to_string(),
        }
    }
}

/// 记忆折叠器
pub struct MemoryCompressor {
    /// 触发压缩的上下文阈值 (token 数)
    threshold: usize,
    /// 目标压缩比例
    target_ratio: f32,
    /// 摘要提示模板
    summary_prompt_template: String,
    /// 模型 Provider (用于生成摘要)
    provider: Arc<dyn Provider>,
    /// 记忆存储
    memory: Arc<dyn Memory>,
}

impl MemoryCompressor {
    /// 创建新的记忆折叠器
    pub fn new(
        config: MemoryCompressorConfig,
        provider: Arc<dyn Provider>,
        memory: Arc<dyn Memory>,
    ) -> Self {
        Self {
            threshold: config.threshold,
            target_ratio: config.target_ratio,
            summary_prompt_template: config.summary_prompt_template,
            provider,
            memory,
        }
    }

    /// 检查是否需要压缩
    pub fn should_compress(&self, total_tokens: usize) -> bool {
        total_tokens > self.threshold
    }

    /// 执行记忆折叠
    pub async fn compress(&self, messages: &[crate::providers::ChatMessage]) -> Result<MemoryCapsule> {
        // 1. 生成摘要
        let summary = self.generate_summary(messages).await?;

        // 2. 提取关键信息
        let entities = self.extract_entities(messages).await?;
        let tool_calls = self.summarize_tool_calls(messages);
        let decisions = self.extract_decisions(messages).await?;

        // 3. 计算重要性（这里我们暂时使用一个简单的计算方法，实际中会调用 ImportanceScorer）
        let importance = self.calculate_basic_importance(&entities, &decisions);

        // 4. 构建胶囊
        let current_timestamp = chrono::Utc::now().timestamp();
        let capsule = MemoryCapsule {
            id: uuid::Uuid::new_v4().to_string(),
            time_range: (
                current_timestamp, // 使用当前时间作为开始时间
                current_timestamp, // 使用当前时间作为结束时间
            ),
            summary,
            entities,
            tool_calls,
            decisions,
            importance,
            original_turns: messages.len(),
            compression_ratio: 0.0, // 将在外部计算
        };

        Ok(capsule)
    }

    /// 生成对话摘要
    async fn generate_summary(&self, messages: &[crate::providers::ChatMessage]) -> Result<String> {
        // 构建对话历史字符串
        let conversation_history = messages
            .iter()
            .map(|msg| format!("{}: {}", msg.role, msg.content))
            .collect::<Vec<_>>()
            .join("\n");

        // 替换提示模板中的占位符
        let prompt = self
            .summary_prompt_template
            .replace("{conversation_history}", &conversation_history);

        // 使用模型生成摘要
        let response = self
            .provider
            .simple_chat(&prompt, "", 0.7) // 使用 simple_chat 方法
            .await?;

        Ok(response)
    }

    /// 提取关键实体
    async fn extract_entities(&self, messages: &[crate::providers::ChatMessage]) -> Result<Vec<Entity>> {
        // 这里可以使用专门的实体提取提示或模型
        // 暂时返回空向量，将在后续实现更复杂的实体提取
        Ok(Vec::new())
    }

    /// 摘要工具调用
    fn summarize_tool_calls(&self, messages: &[crate::providers::ChatMessage]) -> Vec<ToolCallSummary> {
        // 从消息中提取工具调用信息
        // 暂时返回空向量，将在后续实现
        Vec::new()
    }

    /// 提取决策点
    async fn extract_decisions(&self, messages: &[crate::providers::ChatMessage]) -> Result<Vec<Decision>> {
        // 这里可以使用专门的决策提取提示或模型
        // 暂时返回空向量，将在后续实现更复杂的决策提取
        Ok(Vec::new())
    }

    /// 计算基本重要性评分
    fn calculate_basic_importance(&self, entities: &[Entity], decisions: &[Decision]) -> f32 {
        // 基于实体和决策的数量进行简单评分
        let entity_score = (entities.len() as f32).min(1.0) * 0.3;
        let decision_score = (decisions.len() as f32).min(1.0) * 0.7;
        (entity_score + decision_score).min(1.0)
    }
}