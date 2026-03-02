//! 经验提炼器 - 从 Agent 执行轨迹中提取可复用的经验
use crate::a2a::gateway::A2AGateway;
use crate::providers::{ChatMessage, Provider};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// 经验胶囊 - 可复用的策略模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceCapsule {
    /// 唯一标识
    pub id: String,
    /// 来源 Agent
    pub source_agent: String,
    /// 任务类型
    pub task_type: String, // 例如："research", "coding", "analysis" 等
    /// 策略模板
    pub strategy: StrategyTemplate,
    /// 触发条件
    pub trigger_conditions: Vec<Condition>,
    /// 预期结果
    pub expected_outcome: String,
    /// 实际结果
    pub actual_outcome: Outcome,
    /// 置信度 (0.0-1.0)
    pub confidence: f32,
    /// 使用次数
    pub usage_count: usize,
    /// 成功次数
    pub success_count: usize,
    /// 创建时间
    pub created_at: i64,
    /// 最后使用时间
    pub last_used_at: i64,
}

/// 策略模板
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StrategyTemplate {
    /// 步骤序列
    pub steps: Vec<StrategyStep>,
    /// 工具调用序列
    pub tool_sequence: Vec<ToolInvocation>,
    /// 决策点
    pub decision_points: Vec<DecisionPoint>,
    /// 回退策略
    pub fallback: Option<Box<StrategyTemplate>>,
}

/// 策略步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyStep {
    pub step_number: u32,
    pub description: String,
    pub required_inputs: Vec<String>,
    pub expected_outputs: Vec<String>,
    pub success_criteria: String,
}

/// 工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInvocation {
    pub tool_name: String,
    pub parameters: serde_json::Value,
    pub execution_order: u32,
    pub success_rate: f32, // 基于历史执行的成功率
}

/// 决策点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionPoint {
    pub question: String,
    pub options: Vec<String>,
    pub chosen_option: String,
    pub rationale: String,
    pub context_factors: Vec<String>,
}

/// 触发条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub parameter: String,
    pub operator: Operator,
    pub value: serde_json::Value,
    pub weight: f32, // 条件的权重
}

/// 操作符
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Contains,
    MatchesRegex,
}

/// 执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Outcome {
    Success,
    PartialSuccess { details: String },
    Failure { reason: String },
}

/// 执行轨迹 - 用于经验提炼的数据源
#[derive(Debug, Clone)]
pub struct ExecutionTrace {
    pub agent_id: String,
    pub task_type: String,
    pub goal: String,
    pub steps: Vec<TraceStep>,
    pub tool_calls: Vec<TraceToolCall>,
    pub final_result: Outcome,
    pub execution_duration_ms: u64,
    pub token_usage: u64,
}

/// 轨迹步骤
#[derive(Debug, Clone)]
pub struct TraceStep {
    pub step_number: u32,
    pub description: String,
    pub input: String,
    pub output: String,
    pub duration_ms: u64,
    pub success: bool,
}

/// 轨迹工具调用
#[derive(Debug, Clone)]
pub struct TraceToolCall {
    pub tool_name: String,
    pub parameters: serde_json::Value,
    pub result: serde_json::Value,
    pub success: bool,
    pub timestamp: i64,
}

/// 经验提炼配置
#[derive(Debug, Clone, Deserialize)]
pub struct ExperienceExtractorConfig {
    /// 最小样本数（生成经验前）
    pub min_samples: usize,
    /// 最低置信度阈值
    pub min_confidence_threshold: f32,
    /// 经验分享的置信度阈值
    pub share_confidence_threshold: f32,
    /// 成功率权重
    pub success_rate_weight: f32,
    /// 通用性权重（适用于不同场景的程度）
    pub generality_weight: f32,
    /// 复杂度惩罚（过于复杂的策略会被降低权重）
    pub complexity_penalty: f32,
}

impl Default for ExperienceExtractorConfig {
    fn default() -> Self {
        Self {
            min_samples: 3,
            min_confidence_threshold: 0.5,
            share_confidence_threshold: 0.7,
            success_rate_weight: 0.4,
            generality_weight: 0.3,
            complexity_penalty: 0.3,
        }
    }
}

/// 经验提炼器
pub struct ExperienceExtractor {
    /// 模型 Provider (用于分析执行轨迹)
    provider: Arc<dyn Provider>,
    /// A2A 网关 (用于分享经验)
    a2a_gateway: Arc<A2AGateway>,
    /// 配置
    config: ExperienceExtractorConfig,
}

impl ExperienceExtractor {
    /// 创建新的经验提炼器
    pub fn new(
        provider: Arc<dyn Provider>,
        a2a_gateway: Arc<A2AGateway>,
        config: ExperienceExtractorConfig,
    ) -> Self {
        Self {
            provider,
            a2a_gateway,
            config,
        }
    }

    /// 从执行轨迹提取经验
    pub async fn extract(&self, trace: &ExecutionTrace) -> Result<Option<ExperienceCapsule>> {
        // 1. 分析执行结果
        if !self.is_successful(trace) && !self.should_extract_failure_lesson(trace) {
            return Ok(None); // 失败的执行如果没有价值则不生成经验
        }

        // 2. 提取关键步骤
        let key_steps = self.identify_key_steps(trace).await?;

        // 3. 识别触发条件
        let conditions = self.extract_conditions(trace).await?;

        // 4. 生成策略模板
        let strategy = self.generate_strategy(trace, &key_steps).await?;

        // 5. 计算置信度
        let confidence = self.calculate_confidence(trace, &strategy, &conditions).await;

        if confidence < self.config.min_confidence_threshold {
            return Ok(None); // 置信度过低，不生成经验
        }

        // 6. 创建经验胶囊
        let capsule = ExperienceCapsule {
            id: uuid::Uuid::new_v4().to_string(),
            source_agent: trace.agent_id.clone(),
            task_type: trace.task_type.clone(),
            strategy,
            trigger_conditions: conditions,
            expected_outcome: trace.goal.clone(),
            actual_outcome: trace.final_result.clone(),
            confidence,
            usage_count: 0,
            success_count: 0,
            created_at: chrono::Utc::now().timestamp(),
            last_used_at: 0,
        };

        // 7. 如果置信度足够高，通过 A2A 分享
        if confidence >= self.config.share_confidence_threshold {
            self.share_experience(&capsule).await?;
        }

        Ok(Some(capsule))
    }

    /// 提取失败教训
    pub async fn extract_failure_lesson(&self, trace: &ExecutionTrace) -> Result<Option<ExperienceCapsule>> {
        if !matches!(trace.final_result, Outcome::Failure { .. }) {
            return Ok(None); // 只处理失败的情况
        }

        // 分析失败原因，生成避免策略
        let failure_analysis = self.analyze_failure(trace).await?;

        let strategy = StrategyTemplate {
            steps: vec![],
            tool_sequence: vec![],
            decision_points: vec![],
            fallback: None,
        };

        let conditions = failure_analysis.warning_signs
            .into_iter()
            .map(|warning| Condition {
                parameter: warning.parameter,
                operator: Operator::Contains,
                value: serde_json::Value::String(warning.indicator),
                weight: warning.importance,
            })
            .collect();

        let confidence = 0.7; // 失败教训通常也很有价值

        let capsule = ExperienceCapsule {
            id: uuid::Uuid::new_v4().to_string(),
            source_agent: trace.agent_id.clone(),
            task_type: format!("{}_avoidance", trace.task_type),
            strategy,
            trigger_conditions: conditions,
            expected_outcome: format!("Avoid {}", trace.goal),
            actual_outcome: trace.final_result.clone(),
            confidence,
            usage_count: 0,
            success_count: 0,
            created_at: chrono::Utc::now().timestamp(),
            last_used_at: 0,
        };

        Ok(Some(capsule))
    }

    /// 判断执行是否成功
    fn is_successful(&self, trace: &ExecutionTrace) -> bool {
        matches!(trace.final_result, Outcome::Success | Outcome::PartialSuccess { .. })
    }

    /// 判断是否应该提取失败教训
    fn should_extract_failure_lesson(&self, trace: &ExecutionTrace) -> bool {
        matches!(trace.final_result, Outcome::Failure { .. }) && trace.steps.len() > 1
    }

    /// 识别关键步骤
    async fn identify_key_steps(&self, trace: &ExecutionTrace) -> Result<Vec<TraceStep>> {
        // 在实际实现中，这里会使用 AI 模型来分析哪些步骤是关键的
        // 暂时返回所有成功的步骤
        Ok(trace
            .steps
            .iter()
            .filter(|step| step.success)
            .cloned()
            .collect())
    }

    /// 提取触发条件
    async fn extract_conditions(&self, trace: &ExecutionTrace) -> Result<Vec<Condition>> {
        // 在实际实现中，这里会分析任务上下文以确定适用条件
        // 暂时返回一个默认条件
        Ok(vec![Condition {
            parameter: "task_type".to_string(),
            operator: Operator::Equals,
            value: serde_json::Value::String(trace.task_type.clone()),
            weight: 1.0,
        }])
    }

    /// 生成策略模板
    async fn generate_strategy(&self, trace: &ExecutionTrace, key_steps: &[TraceStep]) -> Result<StrategyTemplate> {
        // 在实际实现中，这里会使用 AI 模型来生成可复用的策略
        // 暂时基于轨迹生成一个基本策略
        let steps = key_steps
            .iter()
            .enumerate()
            .map(|(i, step)| StrategyStep {
                step_number: i as u32 + 1,
                description: step.description.clone(),
                required_inputs: vec![], // 在实际实现中会分析输入
                expected_outputs: vec![], // 在实际实现中会分析输出
                success_criteria: step.success.to_string(), // 简化表示
            })
            .collect();

        let tool_calls = trace
            .tool_calls
            .iter()
            .enumerate()
            .map(|(i, call)| ToolInvocation {
                tool_name: call.tool_name.clone(),
                parameters: call.parameters.clone(),
                execution_order: i as u32,
                success_rate: if call.success { 1.0 } else { 0.0 }, // 简化成功率
            })
            .collect();

        Ok(StrategyTemplate {
            steps,
            tool_sequence: tool_calls,
            decision_points: vec![], // 在实际实现中会识别决策点
            fallback: None,
        })
    }

    /// 计算置信度
    async fn calculate_confidence(
        &self,
        trace: &ExecutionTrace,
        strategy: &StrategyTemplate,
        conditions: &[Condition],
    ) -> f32 {
        // 基于多个因素计算置信度
        let success_rate_factor = match &trace.final_result {
            Outcome::Success => 1.0,
            Outcome::PartialSuccess { .. } => 0.7,
            Outcome::Failure { .. } => 0.2,
        };

        // 策略复杂度的惩罚（越复杂，置信度越低，因为泛化能力可能较差）
        let complexity_factor = 1.0 / (1.0 + (strategy.steps.len() as f32 * self.config.complexity_penalty));

        // 条件通用性（条件越具体，适用范围越窄）
        let generality_factor = 1.0 - (conditions.len() as f32 * 0.1).min(0.5);

        // 综合计算
        let combined = (success_rate_factor * self.config.success_rate_weight)
            + (generality_factor * self.config.generality_weight)
            + (complexity_factor * self.config.complexity_penalty);

        combined.min(1.0).max(0.0)
    }

    /// 分析失败原因
    async fn analyze_failure(&self, trace: &ExecutionTrace) -> Result<FailureAnalysis> {
        // 在实际实现中，这里会使用 AI 模型来深入分析失败原因
        // 暂时返回一个基本分析
        Ok(FailureAnalysis {
            reason: match &trace.final_result {
                Outcome::Failure { reason } => reason.clone(),
                _ => "Unknown failure reason".to_string(),
            },
            warning_signs: vec![WarningSign {
                parameter: "execution_time".to_string(),
                indicator: "long_duration".to_string(),
                importance: 0.5,
            }],
        })
    }

    /// 通过 A2A 网关分享经验
    async fn share_experience(&self, capsule: &ExperienceCapsule) -> Result<()> {
        // 创建一个分享经验的消息
        let message_content = serde_json::to_value(capsule)?;
        
        // 使用 A2A 网关发送经验分享消息
        // 注意：这里假设 A2A 网关有相应的方法来处理经验分享
        // 在实际实现中，可能需要扩展 A2AGateway 以支持经验分享
        println!("Sharing experience: {}", capsule.id); // 临时实现
        
        Ok(())
    }
}

/// 失败分析结果
#[derive(Debug, Clone)]
struct FailureAnalysis {
    reason: String,
    warning_signs: Vec<WarningSign>,
}

/// 警告信号
#[derive(Debug, Clone)]
struct WarningSign {
    parameter: String,
    indicator: String,
    importance: f32,
}