//! Skills 模块的兼容层
//! 为现有代码提供向后兼容的接口

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::Value;
use std::path::Path;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// 技能结构定义（兼容现有代码）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// 技能唯一标识符
    pub id: String,
    /// 技能名称
    pub name: String,
    /// 技能描述
    pub description: String,
    /// 技能类别
    pub category: String,
    /// 技能标签
    pub tags: Vec<String>,
    /// 技能版本
    pub version: String,
    /// 执行所需参数
    pub parameters: HashMap<String, ParameterSpec>,
    /// 技能实现路径
    pub implementation: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
    /// 是否启用
    pub enabled: bool,
}

/// 参数规范
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterSpec {
    /// 参数名称
    pub name: String,
    /// 参数类型
    pub param_type: String,  // "string", "number", "boolean", "object", "array"
    /// 是否必需
    pub required: bool,
    /// 默认值
    pub default: Option<Value>,
    /// 参数描述
    pub description: String,
    /// 示例值
    pub example: Option<Value>,
}

/// 加载技能配置的函数（兼容现有代码）
pub fn load_skills_with_config(workspace_dir: &std::path::Path, config: &crate::config::schema::Config) -> Vec<Skill> {
    // 返回空列表作为占位符，以使现有代码能编译通过
    vec![]
}

/// 将技能转换为提示的函数（兼容现有代码）
pub fn skills_to_prompt_with_mode(skills: &[Skill], mode: &str) -> String {
    // 返回空字符串作为占位符
    String::new()
}

/// 加载技能配置的函数（兼容现有代码）
pub fn load_skills_with_config_and_workspace(workspace: &str, config: &serde_json::Value) -> Vec<Skill> {
    // 返回空列表作为占位符
    vec![]
}

/// 加载技能配置的另一个函数（兼容现有代码）
pub fn load_skills_with_config_and_workspace_dir(workspace_dir: &std::path::Path, config: &crate::config::schema::Config) -> Vec<Skill> {
    // 返回空列表作为占位符
    vec![]
}

/// 为现有代码提供一个简单的技能管理器
pub struct SkillManager {
    skills: Arc<RwLock<Vec<Skill>>>,
}

impl SkillManager {
    pub fn new() -> Self {
        Self {
            skills: Arc::new(RwLock::new(vec![])),
        }
    }

    pub async fn list_skills(&self) -> Vec<String> {
        vec![]
    }
}

impl Default for SkillManager {
    fn default() -> Self {
        Self::new()
    }
}