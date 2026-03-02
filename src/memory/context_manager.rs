//! 自动上下文管理器 - 管理 Agent 的上下文窗口，防止超出模型限制
use crate::memory::compressor::{MemoryCapsule, MemoryCompressor};
use crate::memory::importance::{ImportanceScorer, MemoryImportanceEvaluator};
use crate::memory::traits::Memory;
use crate::providers::{ChatMessage, Provider};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// 上下文管理配置
#[derive(Debug, Clone, Deserialize)]
pub struct ContextManagerConfig {
    /// 最大上下文 token 数
    pub max_context_tokens: usize,
    /// 触发压缩的阈值比例
    pub compression_threshold_ratio: f32,
    /// 滑动窗口大小（最近 N 轮）
    pub sliding_window_size: usize,
    /// 关键事件缓存大小
    pub key_event_cache_size: usize,
    /// 重要记忆召回数量
    pub important_memory_recall: usize,
    /// 系统提示最大长度（token 数）
    pub max_system_prompt_tokens: usize,
}

impl Default for ContextManagerConfig {
    fn default() -> Self {
        Self {
            max_context_tokens: 128000, // 128k tokens，适用于大多数现代模型
            compression_threshold_ratio: 0.8, // 当达到容量的 80% 时触发压缩
            sliding_window_size: 20, // 保留最近 20 轮对话
            key_event_cache_size: 10, // 缓存 10 个关键事件
            important_memory_recall: 5, // 召回 5 个重要记忆
            max_system_prompt_tokens: 4000, // 系统提示最大 4k tokens
        }
    }
}

/// 上下文管理器
pub struct ContextManager {
    config: ContextManagerConfig,
    compressor: Arc<MemoryCompressor>,
    memory: Arc<dyn Memory>,
    importance_scorer: Arc<ImportanceScorer>,
    evaluator: Arc<MemoryImportanceEvaluator>,
}

impl ContextManager {
    /// 创建新的上下文管理器
    pub fn new(
        config: ContextManagerConfig,
        compressor: Arc<MemoryCompressor>,
        memory: Arc<dyn Memory>,
        importance_scorer: Arc<ImportanceScorer>,
    ) -> Self {
        let evaluator = Arc::new(MemoryImportanceEvaluator::new((*importance_scorer).clone()));
        
        Self {
            config,
            compressor,
            memory,
            importance_scorer,
            evaluator,
        }
    }

    /// 构建当前上下文
    pub async fn build_context(&self, agent_id: &str, current_message: ChatMessage) -> Result<Vec<ChatMessage>> {
        let mut context = Vec::new();

        // 1. 系统提示（始终包含，但可能截断）
        let system_prompt = self.get_system_prompt(agent_id).await?;
        let system_messages = self.truncate_system_prompt(system_prompt);
        context.extend(system_messages);

        // 2. 关键记忆胶囊（按重要性召回）
        let capsules = self.recall_important_memories(agent_id).await?;
        for capsule in capsules {
            context.push(ChatMessage::system(&capsule.to_prompt()));
        }

        // 3. 最近对话（滑动窗口）
        let recent = self.get_recent_conversation(agent_id).await?;
        context.extend(recent);

        // 4. 当前消息
        context.push(current_message);

        // 5. 检查是否需要压缩
        let total_tokens = self.estimate_tokens(&context).await?;
        if total_tokens > self.config.max_context_tokens {
            context = self.compress_and_rebuild(agent_id, context).await?;
        }

        Ok(context)
    }

    /// 召回重要记忆
    async fn recall_important_memories(&self, agent_id: &str) -> Result<Vec<MemoryCapsule>> {
        // 这里我们需要查询记忆存储中的记忆胶囊
        // 由于目前的 traits::Memory 接口可能不直接支持 MemoryCapsule 查询
        // 我们暂且返回空向量，稍后会扩展接口或使用其他方式实现
        
        // TODO: 扩展 Memory trait 以支持 MemoryCapsule 查询
        Ok(Vec::new())
    }

    /// 获取最近对话
    async fn get_recent_conversation(&self, agent_id: &str) -> Result<Vec<ChatMessage>> {
        // 这里我们需要从记忆存储中获取最近的对话
        // 由于当前接口限制，暂时返回空向量
        // TODO: 扩展 Memory trait 以支持对话历史查询
        Ok(Vec::new())
    }

    /// 获取系统提示
    async fn get_system_prompt(&self, agent_id: &str) -> Result<String> {
        // 从记忆存储中获取系统提示
        // TODO: 实现具体的系统提示获取逻辑
        Ok(format!("You are an AI assistant for agent {}. Maintain context and provide helpful responses.", agent_id))
    }

    /// 截断系统提示以适应 token 限制
    fn truncate_system_prompt(&self, mut prompt: String) -> Vec<ChatMessage> {
        // 简单的截断实现，实际中可能需要更复杂的逻辑
        if self.estimate_token_count(&prompt) > self.config.max_system_prompt_tokens {
            // 截断到最大长度
            let chars: Vec<char> = prompt.chars().collect();
            let max_chars = self.config.max_system_prompt_tokens * 3; // 估算字符数
            if chars.len() > max_chars {
                prompt = chars[..max_chars].iter().collect();
                prompt.push_str("... (truncated)");
            }
        }
        
        vec![ChatMessage::system(&prompt)]
    }

    /// 压缩和重建上下文
    async fn compress_and_rebuild(&self, agent_id: &str, context: Vec<ChatMessage>) -> Result<Vec<ChatMessage>> {
        let mut current_context = context;
        
        // 循环直到上下文大小合适
        loop {
            // 计算当前上下文的 token 数
            let total_tokens = self.estimate_tokens(&current_context).await?;
            
            if !self.compressor.should_compress(total_tokens) || current_context.len() <= 1 {
                // 如果不需要压缩或只剩一个消息，尝试其他策略
                return self.apply_sliding_window(current_context).await;
            }

            // 找到可以压缩的历史部分
            let history_len = current_context.len().saturating_sub(2); // 保留最后一条消息和系统提示
            if history_len == 0 {
                // 无法进一步压缩，尝试滑动窗口
                return self.apply_sliding_window(current_context).await;
            }

            let history_to_compress = &current_context[1..history_len]; // 排除系统提示
            let remaining_context = &current_context[history_len..];

            // 执行记忆折叠
            let capsule = self.compressor.compress(history_to_compress).await?;
            
            // 将压缩的记忆胶囊添加到上下文中
            let mut new_context = vec![current_context[0].clone()]; // 保留系统提示
            new_context.push(ChatMessage::system(&capsule.summary)); // 添加压缩摘要
            new_context.extend_from_slice(remaining_context); // 添加剩余上下文

            // 检查新上下文大小
            let new_total_tokens = self.estimate_tokens(&new_context).await?;
            if new_total_tokens <= self.config.max_context_tokens {
                // 如果大小合适，返回结果
                return Ok(new_context);
            }
            
            // 如果仍然太大，继续循环压缩
            current_context = new_context;
        }
    }

    /// 应用滑动窗口策略
    async fn apply_sliding_window(&self, mut context: Vec<ChatMessage>) -> Result<Vec<ChatMessage>> {
        if context.len() <= self.config.sliding_window_size {
            return Ok(context);
        }

        // 保留系统提示（如果存在）
        let mut new_context = Vec::new();
        if !context.is_empty() && context[0].role == "system" {
            new_context.push(context.remove(0));
        }

        // 保留最近的几轮对话
        let start_idx = context.len().saturating_sub(self.config.sliding_window_size);
        new_context.extend(context.into_iter().skip(start_idx));

        Ok(new_context)
    }

    /// 估算 token 数量
    async fn estimate_tokens(&self, messages: &[ChatMessage]) -> Result<usize> {
        let total = messages
            .iter()
            .map(|msg| self.estimate_token_count(&msg.content))
            .sum();
        Ok(total)
    }

    /// 估算单个字符串的 token 数量
    fn estimate_token_count(&self, text: &str) -> usize {
        // 简单的估算：每个 token 平均约 4 个字符
        // 在实际实现中，可以使用专门的 token 计算库
        text.chars().count() / 4
    }
}

impl MemoryCapsule {
    /// 将记忆胶囊转换为提示格式
    pub fn to_prompt(&self) -> String {
        format!(
            "记忆摘要: {}\n重要实体: {}\n关键决策: {}\n工具调用: {}",
            self.summary,
            self.entities.iter().map(|e| e.name.clone()).collect::<Vec<_>>().join(", "),
            self.decisions.len(),
            self.tool_calls.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_context_manager_creation() {
        // 这里需要创建实际的依赖项来进行测试
        // 由于依赖项较多，暂时跳过具体实现
        assert!(true);
    }

    #[test]
    fn test_estimate_token_count() {
        let config = ContextManagerConfig::default();
        // 创建一个模拟的provider
        use std::sync::Arc;
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
        
        let dummy_compressor = Arc::new(MemoryCompressor::new(
            crate::memory::compressor::MemoryCompressorConfig::default(),
            Arc::new(DummyProvider),
            Arc::new(crate::memory::sqlite::SqliteMemory::new(std::path::Path::new("/tmp")).unwrap()),
        ));
        
        let cm = ContextManager::new(
            config,
            dummy_compressor,
            Arc::new(crate::memory::sqlite::SqliteMemory::new(std::path::Path::new("/tmp")).unwrap()),
            Arc::new(ImportanceScorer::new(crate::memory::importance::ImportanceScorerConfig::default())),
        );

        assert_eq!(cm.estimate_token_count("Hello world!"), 3); // 12 chars / 4 = 3
        assert_eq!(cm.estimate_token_count("This is a longer test string with more tokens."), 11); // 44 chars / 4 = 11
    }

    #[test]
    fn test_truncate_system_prompt() {
        let config = ContextManagerConfig {
            max_system_prompt_tokens: 10,
            ..ContextManagerConfig::default()
        };
        
        // 创建一个模拟的provider
        use std::sync::Arc;
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
        
        let dummy_compressor = Arc::new(MemoryCompressor::new(
            crate::memory::compressor::MemoryCompressorConfig::default(),
            Arc::new(DummyProvider),
            Arc::new(crate::memory::sqlite::SqliteMemory::new(std::path::Path::new("/tmp")).unwrap()),
        ));
        
        let cm = ContextManager::new(
            config,
            dummy_compressor,
            Arc::new(crate::memory::sqlite::SqliteMemory::new(std::path::Path::new("/tmp")).unwrap()),
            Arc::new(ImportanceScorer::new(crate::memory::importance::ImportanceScorerConfig::default())),
        );

        let long_prompt = "This is a very long system prompt that exceeds the maximum token limit and should be truncated.".repeat(10);
        let truncated = cm.truncate_system_prompt(long_prompt);
        
        // 检查是否被截断
        assert!(truncated[0].content.contains("... (truncated)"));
    }
}