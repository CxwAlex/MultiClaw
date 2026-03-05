// src/skills/create_company.rs
use crate::skills::{Skill, SkillExecutor, SkillMetadata, SkillContext, SkillExecutionResult, ExecutionStatus};
use crate::instance::{InstanceManager, InstanceConfig, CreateInstanceRequest, InstanceType, ResourceQuota, CEOConfig};
use crate::instance::ConfigManager;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// 创建公司技能
pub struct CreateCompanySkill {
    instance_manager: Arc<InstanceManager>,
    config_manager: Arc<ConfigManager>,
    metadata: SkillMetadata,
}

impl CreateCompanySkill {
    pub fn new(instance_manager: Arc<InstanceManager>, config_manager: Arc<ConfigManager>) -> Self {
        Self {
            instance_manager,
            config_manager,
            metadata: SkillMetadata {
                name: "create_company".to_string(),
                description: "创建新的公司实例（独立的 MultiClaw 实例）".to_string(),
                skill_type: crate::skills::SkillType::CEO,
                version: "1.0.0".to_string(),
                required_executor_type: crate::skills::ExecutorType::Chairman,
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "公司名称"
                        },
                        "company_type": {
                            "type": "string",
                            "enum": ["market_research", "product_development", "customer_service", "data_analysis", "general", "custom"],
                            "description": "公司类型"
                        },
                        "token_quota": {
                            "type": "integer",
                            "minimum": 10000,
                            "maximum": 10000000,
                            "description": "Token 配额（每分钟）"
                        },
                        "max_agents": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 100,
                            "description": "最大 Agent 数量"
                        },
                        "ceo_model": {
                            "type": "string",
                            "description": "CEO 使用的模型"
                        },
                        "ceo_personality": {
                            "type": "string",
                            "enum": ["analytical", "creative", "strategic", "practical"],
                            "description": "CEO 性格特征"
                        },
                        "channel": {
                            "type": "string",
                            "description": "绑定的通信渠道（可选）"
                        },
                        "base_data_dir": {
                            "type": "string",
                            "description": "基础数据目录"
                        }
                    },
                    "required": ["name", "company_type", "token_quota", "max_agents", "ceo_model", "ceo_personality"]
                }),
                output_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "instance_id": { "type": "string" },
                        "instance_name": { "type": "string" },
                        "instance_type": { "type": "string" },
                        "port": { "type": "integer" },
                        "data_dir": { "type": "string" },
                        "initial_status": { "type": "string" },
                        "message": { "type": "string" }
                    }
                }),
                resource_requirements: crate::skills::ResourceRequirements {
                    compute: Some(5),
                    memory: Some(128),
                    storage: Some(10),
                    bandwidth: Some(100),
                    api_calls: Some(2),
                    tokens: Some(100),
                    concurrent_agents: Some(1),
                },
                created_at: Utc::now(),
                updated_at: Utc::now(),
                author: "MultiClaw System".to_string(),
                tags: vec!["company".to_string(), "creation".to_string(), "management".to_string()],
                category: "Infrastructure".to_string(),
            },
        }
    }
}

#[async_trait::async_trait]
impl SkillExecutor for CreateCompanySkill {
    async fn execute(&self, context: SkillContext) -> Result<SkillExecutionResult, Box<dyn std::error::Error>> {
        // 解析输入参数
        let name = context.inputs.get("name")
            .and_then(|v| v.as_str())
            .ok_or("缺少公司名称")?
            .to_string();
        
        let company_type_str = context.inputs.get("company_type")
            .and_then(|v| v.as_str())
            .ok_or("缺少公司类型")?;
        
        let company_type = match company_type_str {
            "market_research" => InstanceType::MarketResearch,
            "product_development" => InstanceType::ProductDevelopment,
            "customer_service" => InstanceType::CustomerService,
            "data_analysis" => InstanceType::DataAnalysis,
            "general" => InstanceType::General,
            "custom" => InstanceType::Custom,
            _ => return Err("无效的公司类型".into()),
        };
        
        let token_quota = context.inputs.get("token_quota")
            .and_then(|v| v.as_u64())
            .ok_or("缺少 Token 配额")?;
        
        let max_agents = context.inputs.get("max_agents")
            .and_then(|v| v.as_u64())
            .ok_or("缺少最大 Agent 数量")? as u32;
        
        let ceo_model = context.inputs.get("ceo_model")
            .and_then(|v| v.as_str())
            .ok_or("缺少 CEO 模型")?
            .to_string();
        
        let ceo_personality = context.inputs.get("ceo_personality")
            .and_then(|v| v.as_str())
            .ok_or("缺少 CEO 性格")?
            .to_string();
        
        let base_data_dir = context.inputs.get("base_data_dir")
            .and_then(|v| v.as_str())
            .unwrap_or("~/.multiclaw"); // 默认值
        
        let channel = context.inputs.get("channel")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // 构建资源配额
        let resource_quota = ResourceQuota {
            tokens_per_minute: token_quota as u32,
            max_concurrent_agents: max_agents,
            storage_limit_mb: 1000, // 默认 1GB
            api_calls_per_minute: 1000, // 默认 1000 次/分钟
        };

        // 构建 CEO 配置
        let ceo_config = CEOConfig {
            model_preference: ceo_model,
            personality: ceo_personality,
            resource_limits: resource_quota.clone(),
        };

        // 构建创建请求
        let create_request = CreateInstanceRequest {
            name,
            instance_type: company_type,
            quota: resource_quota,
            ceo_config,
            ceo_channel: channel,
            base_data_dir: shellexpand::tilde(base_data_dir).to_string(),
        };

        // 执行创建实例
        let instance_id = self.instance_manager.create_instance(create_request).await?;

        // 获取实例状态
        let status = self.instance_manager.get_instance_status(&instance_id).await
            .unwrap_or(crate::instance::InstanceStatus::Initializing);

        // 返回结果
        let result = serde_json::json!({
            "instance_id": instance_id,
            "instance_name": context.inputs.get("name").unwrap_or(&Value::String("Unknown".to_string())),
            "instance_type": context.inputs.get("company_type").unwrap_or(&Value::String("general".to_string())),
            "port": 8000, // 这个会在创建过程中动态分配
            "data_dir": format!("{}/instances/{}", shellexpand::tilde(base_data_dir), instance_id),
            "initial_status": format!("{:?}", status),
            "message": format!("公司实例「{}」创建成功，ID: {}", 
                context.inputs.get("name").unwrap_or(&Value::String("Unknown".to_string())),
                instance_id)
        });

        Ok(SkillExecutionResult {
            execution_id: Uuid::new_v4().to_string(),
            skill_id: self.metadata.name.clone(),
            status: ExecutionStatus::Success,
            result: Some(result),
            error: None,
            execution_time_ms: 0, // 这里应该计算实际执行时间
            resources_used: Default::default(), // 这里应该记录实际资源使用
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

/// 公司创建引导技能 - 交互式创建流程
pub struct CompanyCreationGuideSkill {
    instance_manager: Arc<InstanceManager>,
    config_manager: Arc<ConfigManager>,
    metadata: SkillMetadata,
}

impl CompanyCreationGuideSkill {
    pub fn new(instance_manager: Arc<InstanceManager>, config_manager: Arc<ConfigManager>) -> Self {
        Self {
            instance_manager,
            config_manager,
            metadata: SkillMetadata {
                name: "company_creation_guide".to_string(),
                description: "引导用户完成公司创建的交互式流程".to_string(),
                skill_type: crate::skills::SkillType::CEO,
                version: "1.0.0".to_string(),
                required_executor_type: crate::skills::ExecutorType::Chairman,
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "step": {
                            "type": "string",
                            "enum": ["init", "name", "type", "resources", "ceo", "channel", "confirm", "complete"],
                            "description": "当前步骤"
                        },
                        "current_data": {
                            "type": "object",
                            "description": "当前收集到的数据"
                        }
                    },
                    "required": ["step"]
                }),
                output_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "next_step": { "type": "string" },
                        "prompt": { "type": "string" },
                        "collected_data": { "type": "object" },
                        "completed": { "type": "boolean" }
                    }
                }),
                resource_requirements: crate::skills::ResourceRequirements {
                    compute: Some(3),
                    memory: Some(64),
                    storage: Some(5),
                    bandwidth: Some(50),
                    api_calls: Some(1),
                    tokens: Some(50),
                    concurrent_agents: Some(1),
                },
                created_at: Utc::now(),
                updated_at: Utc::now(),
                author: "MultiClaw System".to_string(),
                tags: vec!["guide".to_string(), "interactive".to_string(), "company".to_string()],
                category: "Workflow".to_string(),
            },
        }
    }

    /// 获取下一步骤
    fn get_next_step(&self, current_step: &str) -> &'static str {
        match current_step {
            "init" => "name",
            "name" => "type",
            "type" => "resources",
            "resources" => "ceo",
            "ceo" => "channel",
            "channel" => "confirm",
            "confirm" => "complete",
            _ => "complete",
        }
    }
}

#[async_trait::async_trait]
impl SkillExecutor for CompanyCreationGuideSkill {
    async fn execute(&self, context: SkillContext) -> Result<SkillExecutionResult, Box<dyn std::error::Error>> {
        let step = context.inputs.get("step")
            .and_then(|v| v.as_str())
            .ok_or("缺少步骤信息")?;
        
        let current_data = context.inputs.get("current_data")
            .unwrap_or(&Value::Object(serde_json::Map::new()))
            .clone();

        let (next_step, prompt, completed) = match step {
            "init" => (
                "name",
                "欢迎使用公司创建向导！首先，请告诉我您想创建的公司名称：".to_string(),
                false
            ),
            "name" => {
                let name = context.inputs.get("company_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("新公司");
                
                (
                    "type",
                    format!("好的，公司名称为「{}」。\n请选择公司类型：\n1. 市场调研\n2. 产品开发\n3. 客户服务\n4. 数据分析\n5. 通用型\n请回复数字或类型名称：", name),
                    false
                )
            },
            "type" => {
                let company_type = context.inputs.get("company_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("通用型");
                
                (
                    "resources",
                    format!("公司类型：{}。\n请设置资源配额：\n- 每分钟 Token 配额（建议 50000-500000）：\n- 最大 Agent 数量（建议 5-50）：\n请分别提供两个数值，用逗号分隔：", company_type),
                    false
                )
            },
            "resources" => {
                let tokens = context.inputs.get("token_quota")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(100000);
                
                let agents = context.inputs.get("max_agents")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(10);
                
                (
                    "ceo",
                    format!("资源配置：{} tokens/min, {} agents。\n请选择 CEO 的模型偏好：\n1. GPT-4 (通用)\n2. Claude Sonnet (分析)\n3. Gemini Pro (创新)\n请选择并描述 CEO 性格特征（分析型/创意型/战略型/实用型）：", tokens, agents),
                    false
                )
            },
            "ceo" => {
                let model = context.inputs.get("ceo_model")
                    .and_then(|v| v.as_str())
                    .unwrap_or("gpt-4");
                
                let personality = context.inputs.get("ceo_personality")
                    .and_then(|v| v.as_str())
                    .unwrap_or("analytical");
                
                (
                    "channel",
                    format!("CEO 配置：模型={}，性格={}。\n是否需要为该公司绑定通信渠道？\n如需绑定，请提供渠道类型和凭证（如：telegram:YOUR_BOT_TOKEN）；如不需要，请回复“跳过”：", model, personality),
                    false
                )
            },
            "channel" => {
                (
                    "confirm",
                    format!("即将完成配置，确认信息如下：\n{}\n\n请确认是否继续创建（回复“确认”或“取消”）：", 
                        serde_json::to_string_pretty(&current_data)?),
                    false
                )
            },
            "confirm" => {
                // 这里实际执行创建
                let name = current_data.get("name").and_then(|v| v.as_str()).unwrap_or("默认公司");
                let company_type_str = current_data.get("type").and_then(|v| v.as_str()).unwrap_or("general");
                
                let company_type = match company_type_str {
                    "market_research" => InstanceType::MarketResearch,
                    "product_development" => InstanceType::ProductDevelopment,
                    "customer_service" => InstanceType::CustomerService,
                    "data_analysis" => InstanceType::DataAnalysis,
                    "general" => InstanceType::General,
                    "custom" => InstanceType::Custom,
                    _ => InstanceType::General,
                };
                
                let token_quota = current_data.get("token_quota").and_then(|v| v.as_u64()).unwrap_or(100000) as u32;
                let max_agents = current_data.get("max_agents").and_then(|v| v.as_u64()).unwrap_or(10) as u32;
                
                let ceo_model = current_data.get("ceo_model").and_then(|v| v.as_str()).unwrap_or("gpt-4").to_string();
                let ceo_personality = current_data.get("ceo_personality").and_then(|v| v.as_str()).unwrap_or("analytical").to_string();
                
                let channel = current_data.get("channel").and_then(|v| v.as_str()).map(|s| s.to_string());
                
                let resource_quota = ResourceQuota {
                    tokens_per_minute: token_quota,
                    max_concurrent_agents: max_agents,
                    storage_limit_mb: 1000,
                    api_calls_per_minute: 1000,
                };
                
                let ceo_config = CEOConfig {
                    model_preference: ceo_model,
                    personality: ceo_personality,
                    resource_limits: resource_quota.clone(),
                };
                
                let create_request = CreateInstanceRequest {
                    name: name.to_string(),
                    instance_type: company_type,
                    quota: resource_quota,
                    ceo_config,
                    ceo_channel: channel,
                    base_data_dir: shellexpand::tilde("~/.multiclaw").to_string(),
                };
                
                let instance_id = self.instance_manager.create_instance(create_request).await?;
                
                (
                    "complete", 
                    format!("✅ 公司创建成功！\n公司名称：{}\n实例ID：{}\n访问端口：{}（动态分配）\n数据目录：~/.multiclaw/instances/{}/\n\n公司已启动运行！", name, instance_id, 8000, instance_id), 
                    true
                )
            },
            _ => ("init", "欢迎使用公司创建向导！".to_string(), false)
        };

        let result = serde_json::json!({
            "next_step": next_step,
            "prompt": prompt,
            "collected_data": current_data,
            "completed": completed
        });

        Ok(SkillExecutionResult {
            execution_id: Uuid::new_v4().to_string(),
            skill_id: self.metadata.name.clone(),
            status: ExecutionStatus::Success,
            result: Some(result),
            error: None,
            execution_time_ms: 0,
            resources_used: Default::default(),
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