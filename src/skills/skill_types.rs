//! Skills 模块
//! 提供 MultiClaw 的技能编排和管理功能

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde_json::Value;

/// 技能结构定义
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

/// 技能执行器 trait
#[async_trait::async_trait]
pub trait SkillExecutor: Send + Sync {
    /// 执行技能
    async fn execute(&self, params: HashMap<String, Value>) -> Result<Value, Box<dyn std::error::Error>>;
    /// 获取技能元数据
    fn metadata(&self) -> SkillMetadata;
    /// 获取技能名称
    fn name(&self) -> &str;
}

/// 技能元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    /// 技能名称
    pub name: String,
    /// 技能描述
    pub description: String,
    /// 参数规范
    pub parameters: Vec<ParameterSpec>,
    /// 分类
    pub category: String,
    /// 标签
    pub tags: Vec<String>,
    /// 版本
    pub version: String,
}

/// 技能管理器
pub struct SkillManager {
    /// 存储注册的技能
    pub skills: Arc<RwLock<HashMap<String, Arc<dyn SkillExecutor>>>>,
}

impl SkillManager {
    /// 创建新的技能管理器
    pub fn new() -> Self {
        Self {
            skills: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册技能
    pub async fn register_skill(&self, skill: Arc<dyn SkillExecutor>) {
        let mut skills = self.skills.write().await;
        let name = skill.name().to_string();
        skills.insert(name, skill);
    }

    /// 执行技能
    pub async fn execute_skill(&self, name: &str, params: HashMap<String, Value>) -> Result<Value, String> {
        let skills = self.skills.read().await;
        if let Some(skill) = skills.get(name) {
            match skill.execute(params).await {
                Ok(result) => Ok(result),
                Err(e) => Err(e.to_string()),
            }
        } else {
            Err(format!("Skill '{}' not found", name))
        }
    }

    /// 获取技能列表
    pub async fn list_skills(&self) -> Vec<String> {
        let skills = self.skills.read().await;
        skills.keys().cloned().collect()
    }
}

impl Default for SkillManager {
    fn default() -> Self {
        Self::new()
    }
}

// 预定义的一些常用技能

/// 信息收集技能
pub struct InformationGatheringSkill;

#[async_trait::async_trait]
impl SkillExecutor for InformationGatheringSkill {
    async fn execute(&self, params: HashMap<String, Value>) -> Result<Value, Box<dyn std::error::Error>> {
        // 模拟信息收集过程
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // 获取查询参数
        let query = params.get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("default query");
            
        // 模拟返回结果
        let result = serde_json::json!({
            "query": query,
            "results": [
                {
                    "title": "Sample Result 1",
                    "content": "This is sample content for demonstration purposes.",
                    "source": "Demo Source",
                    "relevance_score": 0.95
                }
            ],
            "timestamp": Utc::now().to_rfc3339()
        });

        Ok(result)
    }

    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            name: "information_gathering".to_string(),
            description: "收集和整理相关信息的技能".to_string(),
            parameters: vec![
                ParameterSpec {
                    name: "query".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    default: None,
                    description: "搜索查询".to_string(),
                    example: Some(Value::String("How to build a multi-agent system".to_string())),
                },
                ParameterSpec {
                    name: "max_results".to_string(),
                    param_type: "number".to_string(),
                    required: false,
                    default: Some(Value::Number(serde_json::Number::from(5))),
                    description: "最大结果数".to_string(),
                    example: Some(Value::Number(serde_json::Number::from(10))),
                }
            ],
            category: "Data Collection".to_string(),
            tags: vec!["research".to_string(), "information".to_string()],
            version: "1.0.0".to_string(),
        }
    }

    fn name(&self) -> &str {
        "information_gathering"
    }
}

/// 数据分析技能
pub struct DataAnalysisSkill;

#[async_trait::async_trait]
impl SkillExecutor for DataAnalysisSkill {
    async fn execute(&self, params: HashMap<String, Value>) -> Result<Value, Box<dyn std::error::Error>> {
        // 模拟数据分析过程
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        
        // 获取数据参数
        let data = params.get("data").cloned().unwrap_or(Value::Array(vec![]));
        
        // 模拟分析结果
        let result = serde_json::json!({
            "input_data_points": match &data {
                Value::Array(arr) => arr.len(),
                Value::Object(_) => 1,
                _ => 0,
            },
            "analysis_type": "statistical",
            "insights": [
                "Data shows increasing trend in the first quarter",
                "Correlation found between variables X and Y"
            ],
            "summary": "Analysis completed successfully with meaningful insights",
            "timestamp": Utc::now().to_rfc3339()
        });

        Ok(result)
    }

    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            name: "data_analysis".to_string(),
            description: "分析数据并生成洞察的技能".to_string(),
            parameters: vec![
                ParameterSpec {
                    name: "data".to_string(),
                    param_type: "array".to_string(),
                    required: true,
                    default: Some(Value::Array(vec![])),
                    description: "待分析的数据".to_string(),
                    example: Some(serde_json::json!([{"x": 1, "y": 2}, {"x": 2, "y": 4}])),
                },
                ParameterSpec {
                    name: "analysis_type".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                    default: Some(Value::String("statistical".to_string())),
                    description: "分析类型".to_string(),
                    example: Some(Value::String("trend".to_string())),
                }
            ],
            category: "Data Processing".to_string(),
            tags: vec!["analysis".to_string(), "data".to_string()],
            version: "1.0.0".to_string(),
        }
    }

    fn name(&self) -> &str {
        "data_analysis"
    }
}

/// 文件操作技能
pub struct FileOperationSkill;

#[async_trait::async_trait]
impl SkillExecutor for FileOperationSkill {
    async fn execute(&self, params: HashMap<String, Value>) -> Result<Value, Box<dyn std::error::Error>> {
        // 模拟文件操作过程
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        
        let operation = params.get("operation")
            .and_then(|v| v.as_str())
            .unwrap_or("read");
        
        let path = params.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        let result = match operation {
            "read" => serde_json::json!({
                "success": true,
                "operation": "read",
                "path": path,
                "content": "Simulated file content...",
                "size": 1024
            }),
            "write" => serde_json::json!({
                "success": true,
                "operation": "write",
                "path": path,
                "bytes_written": 1024
            }),
            "list" => serde_json::json!({
                "success": true,
                "operation": "list",
                "path": path,
                "files": ["file1.txt", "file2.doc", "image.png"]
            }),
            _ => serde_json::json!({
                "success": false,
                "error": format!("Unsupported operation: {}", operation)
            })
        };

        Ok(result)
    }

    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            name: "file_operation".to_string(),
            description: "执行文件系统操作的技能".to_string(),
            parameters: vec![
                ParameterSpec {
                    name: "operation".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    default: Some(Value::String("read".to_string())),
                    description: "操作类型 (read/write/list)".to_string(),
                    example: Some(Value::String("read".to_string())),
                },
                ParameterSpec {
                    name: "path".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    default: None,
                    description: "文件路径".to_string(),
                    example: Some(Value::String("/path/to/file.txt".to_string())),
                },
                ParameterSpec {
                    name: "content".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                    default: None,
                    description: "写入内容 (write 操作时)".to_string(),
                    example: Some(Value::String("Hello, world!".to_string())),
                }
            ],
            category: "System Operations".to_string(),
            tags: vec!["file".to_string(), "system".to_string()],
            version: "1.0.0".to_string(),
        }
    }

    fn name(&self) -> &str {
        "file_operation"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_skill_manager() {
        let manager = SkillManager::new();
        
        // 注册技能
        manager.register_skill(Arc::new(InformationGatheringSkill)).await;
        manager.register_skill(Arc::new(DataAnalysisSkill)).await;
        
        // 验证技能已注册
        let skills_list = manager.list_skills().await;
        assert!(skills_list.contains(&"information_gathering".to_string()));
        assert!(skills_list.contains(&"data_analysis".to_string()));
        
        // 执行信息收集技能
        let mut params = HashMap::new();
        params.insert("query".to_string(), Value::String("test query".to_string()));
        
        let result = manager.execute_skill("information_gathering", params).await;
        assert!(result.is_ok());
        
        let result_json = result.unwrap();
        assert!(result_json.get("results").is_some());
        assert_eq!(result_json.get("query").unwrap().as_str().unwrap(), "test query");
    }

    #[tokio::test]
    async fn test_information_gathering_skill() {
        let skill = InformationGatheringSkill;
        let mut params = HashMap::new();
        params.insert("query".to_string(), Value::String("sample query".to_string()));
        
        let result = skill.execute(params).await;
        assert!(result.is_ok());
        
        let result_json = result.unwrap();
        assert!(result_json.get("results").is_some());
        assert_eq!(result_json.get("query").unwrap().as_str().unwrap(), "sample query");
    }

    #[tokio::test]
    async fn test_data_analysis_skill() {
        let skill = DataAnalysisSkill;
        let mut params = HashMap::new();
        params.insert("data".to_string(), serde_json::json!([{"x": 1, "y": 2}, {"x": 2, "y": 4}]));
        
        let result = skill.execute(params).await;
        assert!(result.is_ok());
        
        let result_json = result.unwrap();
        assert!(result_json.get("insights").is_some());
        assert_eq!(result_json.get("input_data_points").unwrap().as_u64().unwrap(), 2);
    }
}