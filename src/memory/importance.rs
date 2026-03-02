//! 重要性评分系统 - 用于评估记忆条目的重要性
use crate::core::MemoryLevel;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 重要性评分配置
#[derive(Debug, Clone, Deserialize)]
pub struct ImportanceScorerConfig {
    /// 评分权重配置
    pub weights: ImportanceWeights,
    /// 时间半衰期（小时）
    pub time_half_life_hours: f64,
    /// 最小重要性阈值
    pub min_importance_threshold: f32,
    /// 最大重要性阈值
    pub max_importance_threshold: f32,
}

impl Default for ImportanceScorerConfig {
    fn default() -> Self {
        Self {
            weights: ImportanceWeights::default(),
            time_half_life_hours: 168.0, // 一周
            min_importance_threshold: 0.1,
            max_importance_threshold: 0.9,
        }
    }
}

/// 重要性评分权重
#[derive(Debug, Clone, Deserialize)]
pub struct ImportanceWeights {
    /// 用户标记重要性的权重
    pub user_marked: f32,
    /// 引用次数的权重
    pub reference: f32,
    /// 工具调用成功率的权重
    pub tool_success: f32,
    /// 决策影响的权重
    pub decision: f32,
    /// 时间衰减的权重
    pub time_decay: f32,
}

impl Default for ImportanceWeights {
    fn default() -> Self {
        Self {
            user_marked: 0.3,
            reference: 0.2,
            tool_success: 0.2,
            decision: 0.3,
            time_decay: 1.0,
        }
    }
}

/// 重要性评分因子
#[derive(Debug, Clone)]
pub struct ImportanceFactors {
    /// 用户明确提及的重要性
    pub user_marked: Option<f32>,
    /// 引用次数
    pub reference_count: usize,
    /// 工具调用成功率
    pub tool_success_rate: f32,
    /// 决策影响范围
    pub decision_impact: f32,
    /// 时间衰减因子
    pub time_decay: f32,
    /// 记忆级别（全局 > 集群 > 团队 > 本地）
    pub memory_level: MemoryLevel,
    /// 是否被标记为关键
    pub is_critical: bool,
}

impl Default for ImportanceFactors {
    fn default() -> Self {
        Self {
            user_marked: None,
            reference_count: 0,
            tool_success_rate: 0.0,
            decision_impact: 0.0,
            time_decay: 1.0,
            memory_level: MemoryLevel::Local,
            is_critical: false,
        }
    }
}

/// 重要性评分器
#[derive(Clone)]
pub struct ImportanceScorer {
    config: ImportanceScorerConfig,
}

impl ImportanceScorer {
    /// 创建新的重要性评分器
    pub fn new(config: ImportanceScorerConfig) -> Self {
        Self { config }
    }

    /// 计算综合重要性评分
    pub fn score(&self, factors: &ImportanceFactors) -> f32 {
        let weights = &self.config.weights;

        // 基础评分计算
        let mut score = 0.0;
        
        // 用户标记的重要性
        score += factors.user_marked.unwrap_or(0.5) * weights.user_marked;
        
        // 引用次数（对数增长，避免过度影响）
        score += (factors.reference_count as f32).ln().min(2.0) * weights.reference;
        
        // 工具调用成功率
        score += factors.tool_success_rate * weights.tool_success;
        
        // 决策影响
        score += factors.decision_impact * weights.decision;
        
        // 应用时间衰减
        score *= factors.time_decay * weights.time_decay;

        // 根据记忆级别调整
        score = self.adjust_for_memory_level(score, factors.memory_level);

        // 如果被标记为关键，则提升重要性
        if factors.is_critical {
            score = score.max(0.8); // 关键记忆至少有 0.8 的重要性
        }

        // 限制在 0.0-1.0 范围内
        score.clamp(0.0, 1.0)
    }

    /// 根据记忆级别调整评分
    fn adjust_for_memory_level(&self, base_score: f32, level: MemoryLevel) -> f32 {
        match level {
            MemoryLevel::Global => base_score * 1.2,   // 全局记忆最重要
            MemoryLevel::Cluster => base_score * 1.1,  // 集群记忆较重要
            MemoryLevel::Team => base_score,            // 团队记忆正常
            MemoryLevel::Local => base_score * 0.9,    // 本地记忆稍低
        }.min(1.0) // 确保不超过 1.0
    }

    /// 时间衰减函数
    pub fn time_decay(&self, age_hours: f64) -> f32 {
        let half_life = self.config.time_half_life_hours;
        2.0_f32.powf(-(age_hours / half_life) as f32)
    }

    /// 更新记忆的引用计数
    pub fn update_reference_count(&self, current_count: usize) -> usize {
        current_count + 1
    }

    /// 计算记忆的老化程度
    pub fn memory_age_factor(&self, creation_time: i64, current_time: i64) -> f32 {
        let age_seconds = current_time - creation_time;
        let age_hours = age_seconds as f64 / 3600.0;
        self.time_decay(age_hours)
    }
}

/// 记忆重要性评估器 - 用于批量评估记忆条目
pub struct MemoryImportanceEvaluator {
    scorer: ImportanceScorer,
    /// 重要性阈值映射
    thresholds: HashMap<MemoryLevel, f32>,
}

impl MemoryImportanceEvaluator {
    pub fn new(scorer: ImportanceScorer) -> Self {
        let mut thresholds = HashMap::new();
        thresholds.insert(MemoryLevel::Global, 0.3);
        thresholds.insert(MemoryLevel::Cluster, 0.4);
        thresholds.insert(MemoryLevel::Team, 0.5);
        thresholds.insert(MemoryLevel::Local, 0.6);

        Self { scorer, thresholds }
    }

    /// 评估记忆是否应该保留
    pub fn should_retain(&self, factors: &ImportanceFactors) -> bool {
        let score = self.scorer.score(factors);
        let threshold = self.thresholds.get(&factors.memory_level).copied().unwrap_or(0.5);
        score >= threshold
    }

    /// 评估记忆是否需要长期保存
    pub fn should_persist_long_term(&self, factors: &ImportanceFactors) -> bool {
        let score = self.scorer.score(factors);
        score >= 0.7 // 高重要性记忆需要长期保存
    }

    /// 获取记忆的保留期限（天）
    pub fn retention_period_days(&self, factors: &ImportanceFactors) -> u32 {
        let score = self.scorer.score(factors);
        
        if score >= 0.8 {
            365 // 非常重要的记忆保留一年
        } else if score >= 0.6 {
            90  // 重要记忆保留三个月
        } else if score >= 0.4 {
            30  // 一般记忆保留一个月
        } else {
            7   // 低重要性记忆保留一周
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_importance_scorer_basic() {
        let config = ImportanceScorerConfig::default();
        let scorer = ImportanceScorer::new(config);
        
        let factors = ImportanceFactors {
            user_marked: Some(0.9),
            reference_count: 10,
            tool_success_rate: 0.95,
            decision_impact: 0.8,
            time_decay: 0.9,
            memory_level: MemoryLevel::Global,
            is_critical: false,
        };
        
        let score = scorer.score(&factors);
        assert!(score > 0.0 && score <= 1.0);
    }

    #[test]
    fn test_time_decay() {
        let config = ImportanceScorerConfig::default();
        let scorer = ImportanceScorer::new(config);
        
        // 记忆刚创建时，衰减因子接近1
        let decay_new = scorer.time_decay(0.0);
        assert!(decay_new >= 0.99);
        
        // 记忆一周后，衰减一半
        let decay_week = scorer.time_decay(168.0); // 168小时 = 1周
        assert!(decay_week <= 0.55 && decay_week >= 0.45);
        
        // 记忆很久后，衰减接近0
        let decay_old = scorer.time_decay(1000.0);
        assert!(decay_old <= 0.03);
    }

    #[test]
    fn test_memory_level_adjustment() {
        let config = ImportanceScorerConfig::default();
        let scorer = ImportanceScorer::new(config);
        
        let mut factors = ImportanceFactors {
            user_marked: Some(0.5),
            reference_count: 0,
            tool_success_rate: 0.0,
            decision_impact: 0.5,
            time_decay: 1.0,
            memory_level: MemoryLevel::Local,
            is_critical: false,
        };
        
        // 本地记忆得分
        let local_score = scorer.score(&factors);
        
        // 团队记忆得分
        factors.memory_level = MemoryLevel::Team;
        let team_score = scorer.score(&factors);
        
        // 集群记忆得分
        factors.memory_level = MemoryLevel::Cluster;
        let cluster_score = scorer.score(&factors);
        
        // 全局记忆得分
        factors.memory_level = MemoryLevel::Global;
        let global_score = scorer.score(&factors);
        
        // 级别越高的记忆，得分应该越高（在其他条件相同的情况下）
        assert!(local_score <= team_score);
        assert!(team_score <= cluster_score);
        assert!(cluster_score <= global_score);
    }

    #[test]
    fn test_critical_flag() {
        let config = ImportanceScorerConfig::default();
        let scorer = ImportanceScorer::new(config);
        
        let factors_normal = ImportanceFactors {
            user_marked: Some(0.2), // 较低的用户评分
            reference_count: 0,
            tool_success_rate: 0.0,
            decision_impact: 0.0,
            time_decay: 1.0,
            memory_level: MemoryLevel::Local,
            is_critical: false,
        };
        
        let factors_critical = ImportanceFactors {
            is_critical: true, // 标记为关键
            ..factors_normal.clone()
        };
        
        let normal_score = scorer.score(&factors_normal);
        let critical_score = scorer.score(&factors_critical);
        
        // 关键记忆的得分应该更高，至少达到0.8
        assert!(critical_score >= 0.8);
        assert!(critical_score > normal_score);
    }
}