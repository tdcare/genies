//! 应用服务层
//!
//! 封装业务用例，协调领域实体与事件发布。

use crate::application::dto::{ChangePasswordRequest, LoginResponse, PageQuery};
use crate::domain::entity::{
    AdminDepartment, AdminPermission, AdminRole, AdminUser, RolePermission, UserDepartment,
    UserRole, UserRoleMapping,
};
use crate::domain::service::{RoleDomainService, UserDomainService};
use genies::context::CONTEXT;
use genies_auth::event::*;

// ============================================================================
// AuthService — 认证相关
// ============================================================================

pub struct AuthService;

impl AuthService {
    /// 用户名密码登录
    pub async fn login(
        username: &str,
        password: &str,
        jwt_secret: &str,
        expires_in_secs: usize,
    ) -> Result<LoginResponse, String> {
        let rb = &CONTEXT.rbatis;

        // 查找用户
        let user = AdminUser::find_by_username(rb, username)
            .await
            .map_err(|e| format!("服务内部错误: {}", e))?
            .ok_or_else(|| "用户名或密码错误".to_string())?;

        // 检查状态
        if user.status != 1 {
            return Err("用户已被禁用".to_string());
        }

        // 验证密码
        match bcrypt::verify(password, &user.password_hash) {
            Ok(true) => {}
            Ok(false) => return Err("用户名或密码错误".to_string()),
            Err(e) => return Err(format!("服务内部错误: {}", e)),
        }

        // 签发 Token
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        let claims = genies_auth::LocalClaims {
            sub: user.username.clone(),
            uid: user.id,
            name: Some(user.display_name.clone()),
            iat: now,
            exp: now + expires_in_secs,
        };

        let token = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret(jwt_secret.as_bytes()),
        )
        .map_err(|e| format!("签发令牌失败: {}", e))?;

        // 更新最后登录时间
        if let Some(uid) = user.id {
            let _ = AdminUser::update_last_login(rb, &uid).await;
        }

        Ok(LoginResponse {
            access_token: token,
            token_type: "Bearer".to_string(),
            expires_in: expires_in_secs,
            username: user.username,
            display_name: user.display_name,
        })
    }

    /// 获取当前用户信息
    pub async fn get_current_user(user_id: i64) -> Result<serde_json::Value, String> {
        let rb = &CONTEXT.rbatis;
        let user = AdminUser::find_by_id(rb, &user_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "用户不存在".to_string())?;

        Ok(serde_json::json!({
            "id": user.id,
            "username": user.username,
            "display_name": user.display_name,
            "email": user.email,
            "phone": user.phone,
            "avatar": user.avatar,
            "status": user.status,
        }))
    }

    /// 修改密码
    pub async fn change_password(user_id: i64, req: &ChangePasswordRequest) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let user = AdminUser::find_by_id(rb, &user_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "用户不存在".to_string())?;

        // 验证旧密码
        match bcrypt::verify(&req.old_password, &user.password_hash) {
            Ok(true) => {}
            _ => return Err("旧密码错误".to_string()),
        }

        // 加密新密码
        let new_hash = bcrypt::hash(&req.new_password, bcrypt::DEFAULT_COST)
            .map_err(|e| format!("密码加密失败: {}", e))?;

        AdminUser::update_password(rb, &user_id, &new_hash)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// 刷新 Token
    pub async fn refresh_token(
        claims: &genies_auth::LocalClaims,
        jwt_secret: &str,
        expires_in_secs: usize,
    ) -> Result<serde_json::Value, String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        let new_claims = genies_auth::LocalClaims {
            sub: claims.sub.clone(),
            uid: claims.uid,
            name: claims.name.clone(),
            iat: now,
            exp: now + expires_in_secs,
        };

        let token = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &new_claims,
            &jsonwebtoken::EncodingKey::from_secret(jwt_secret.as_bytes()),
        )
        .map_err(|e| format!("签发令牌失败: {}", e))?;

        Ok(serde_json::json!({
            "access_token": token,
            "token_type": "Bearer",
            "expires_in": expires_in_secs
        }))
    }
}

// ============================================================================
// UserAppService — 用户管理
// ============================================================================

pub struct UserAppService;

impl UserAppService {
    pub async fn list(query: &PageQuery) -> Result<serde_json::Value, String> {
        let rb = &CONTEXT.rbatis;
        let page = query.page.unwrap_or(1).max(1);
        let size = query.size.unwrap_or(10).min(100);
        let keyword = query.keyword.clone().unwrap_or_default();

        let total = AdminUser::count(rb, &keyword, None)
            .await
            .map_err(|e| e.to_string())?;

        let all = AdminUser::list(rb, &keyword, None)
            .await
            .map_err(|e| e.to_string())?;

        // 内存分页
        let offset = ((page - 1) * size) as usize;
        let list: Vec<_> = all.into_iter().skip(offset).take(size as usize).collect();

        Ok(serde_json::json!({
            "total": total,
            "page": page,
            "size": size,
            "list": list
        }))
    }

    pub async fn create(input: &serde_json::Value) -> Result<serde_json::Value, String> {
        let rb = &CONTEXT.rbatis;
        let username = input["username"].as_str().unwrap_or("").to_string();
        let password = input["password"].as_str().unwrap_or("123456").to_string();
        let display_name = input["display_name"].as_str().unwrap_or(&username).to_string();
        let email = input["email"].as_str().unwrap_or("").to_string();
        let phone = input["phone"].as_str().unwrap_or("").to_string();

        // 检查用户名唯一
        if let Ok(Some(_)) = AdminUser::find_by_username(rb, &username).await {
            return Err("用户名已存在".to_string());
        }

        let password_hash = bcrypt::hash(&password, bcrypt::DEFAULT_COST)
            .map_err(|e| format!("密码加密失败: {}", e))?;

        let user = AdminUser {
            id: None,
            username: username.clone(),
            password_hash: password_hash.clone(),
            display_name: display_name.clone(),
            email: Some(email.clone()),
            phone: Some(phone.clone()),
            avatar: None,
            status: 1,
            last_login_at: None,
            created_at: None,
            updated_at: None,
        };

        // 构造事件（ID 稍后通过查询获取）
        let event = UserCreatedEvent {
            id: 0, // 占位，insert 后回填
            username: username.clone(),
            password_hash: password_hash.clone(),
            display_name: display_name.clone(),
            email: email.clone(),
            phone: phone.clone(),
            department_id: String::new(),
            department_name: String::new(),
            status: 1,
        };

        UserDomainService::create(&user, event).await?;

        // 获取新创建的用户ID
        let new_user = AdminUser::find_by_username(rb, &username)
            .await
            .ok()
            .flatten()
            .unwrap_or(user);

        Ok(serde_json::json!({"id": new_user.id}))
    }

    pub async fn get_by_id(id: i64) -> Result<AdminUser, String> {
        let rb = &CONTEXT.rbatis;
        AdminUser::find_by_id(rb, &id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "用户不存在".to_string())
    }

    pub async fn update(id: i64, input: &serde_json::Value) -> Result<(), String> {
        let username = input["username"].as_str().unwrap_or("").to_string();
        let display_name = input["display_name"].as_str().unwrap_or("").to_string();
        let email = input["email"].as_str().unwrap_or("").to_string();
        let phone = input["phone"].as_str().unwrap_or("").to_string();
        let status: i8 = input["status"].as_i64().unwrap_or(1) as i8;

        let event = UserUpdatedEvent {
            id,
            username: username.clone(),
            password_hash: String::new(),
            display_name: display_name.clone(),
            email: email.clone(),
            phone: phone.clone(),
            department_id: String::new(),
            department_name: String::new(),
            status,
        };

        UserDomainService::update(id, &username, &display_name, &email, &phone, status, event).await?;

        Ok(())
    }

    pub async fn delete(id: i64) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let user = AdminUser::find_by_id(rb, &id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "用户不存在".to_string())?;

        let event = UserDeletedEvent {
            id,
            username: user.username,
        };

        UserDomainService::delete(id, event).await?;

        Ok(())
    }

    pub async fn batch_delete(ids: &[i64]) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        AdminUser::batch_delete(rb, ids).await.map(|_| ()).map_err(|e| e.to_string())
    }

    pub async fn update_status(id: i64, status: i8) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        AdminUser::update_status(rb, &id, &status).await.map(|_| ()).map_err(|e| e.to_string())
    }

    pub async fn reset_password(id: i64, new_password: &str) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let hash = bcrypt::hash(new_password, bcrypt::DEFAULT_COST)
            .map_err(|e| format!("密码加密失败: {}", e))?;
        AdminUser::update_password(rb, &id, &hash).await.map(|_| ()).map_err(|e| e.to_string())
    }

    pub async fn get_user_roles(user_id: i64) -> Result<Vec<AdminRole>, String> {
        let rb = &CONTEXT.rbatis;
        UserRole::list_by_user(rb, &user_id).await.map_err(|e| e.to_string())
    }

    pub async fn assign_role(user_id: i64, role_id: i64) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let user = AdminUser::find_by_id(rb, &user_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "用户不存在".to_string())?;
        let role = AdminRole::find_by_id(rb, &role_id).await.ok().flatten();
        let role_name = role.map(|r| r.name).unwrap_or_default();
        let event = UserRoleAssignedEvent {
            user_id,
            username: user.username.clone(),
            role_id,
            role_name: role_name.clone(),
        };

        UserDomainService::assign_role(user_id, role_id, event).await?;

        Ok(())
    }

    pub async fn revoke_role(user_id: i64, role_id: i64) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let user = AdminUser::find_by_id(rb, &user_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "用户不存在".to_string())?;
        let role = AdminRole::find_by_id(rb, &role_id).await.ok().flatten();
        let role_name = role.map(|r| r.name).unwrap_or_default();

        let event = UserRoleRevokedEvent {
            user_id,
            username: user.username.clone(),
            role_id,
            role_name: role_name.clone(),
        };

        UserDomainService::revoke_role(user_id, role_id, event).await?;

        Ok(())
    }

    pub async fn get_user_permissions(user_id: i64) -> Result<Vec<AdminPermission>, String> {
        let rb = &CONTEXT.rbatis;
        RolePermission::list_by_user(rb, &user_id).await.map_err(|e| e.to_string())
    }
}

// ============================================================================
// RoleAppService — 角色管理
// ============================================================================

pub struct RoleAppService;

impl RoleAppService {
    pub async fn list_all() -> Result<Vec<AdminRole>, String> {
        let rb = &CONTEXT.rbatis;
        AdminRole::list_all(rb).await.map_err(|e| e.to_string())
    }

    pub async fn create(input: &serde_json::Value) -> Result<serde_json::Value, String> {
        let rb = &CONTEXT.rbatis;
        let name = input["name"].as_str().unwrap_or("").to_string();
        let display_name = input["display_name"].as_str().unwrap_or(&name).to_string();
        let description = input["description"].as_str().unwrap_or("").to_string();

        if name.is_empty() {
            return Err("角色标识不能为空".to_string());
        }

        let role = AdminRole {
            id: None,
            name: name.clone(),
            display_name: display_name.clone(),
            description: Some(description.clone()),
            parent_id: input["parent_id"].as_i64(),
            status: input["status"].as_i64().unwrap_or(1) as i8,
            created_at: None,
            updated_at: None,
        };

        let event = RoleCreatedEvent {
            id: 0, // 占位，insert 后回填
            name,
            display_name,
            description,
            status: role.status,
        };

        RoleDomainService::create(&role, event).await?;

        let new_role = AdminRole::find_by_name(rb, &role.name).await.ok().flatten().unwrap_or(role);
        Ok(serde_json::json!({"id": new_role.id}))
    }

    pub async fn get_by_id(id: i64) -> Result<AdminRole, String> {
        let rb = &CONTEXT.rbatis;
        AdminRole::find_by_id(rb, &id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "角色不存在".to_string())
    }

    pub async fn update(id: i64, input: &serde_json::Value) -> Result<(), String> {
        let name = input["name"].as_str().unwrap_or("").to_string();
        let display_name = input["display_name"].as_str().unwrap_or("").to_string();
        let description = input["description"].as_str().unwrap_or("").to_string();
        let status: i8 = input["status"].as_i64().unwrap_or(1) as i8;

        let event = RoleUpdatedEvent {
            id, name: name.clone(), display_name: display_name.clone(),
            description: description.clone(), status,
        };

        RoleDomainService::update(id, &name, &display_name, &description, status, event).await?;

        Ok(())
    }

    pub async fn delete(id: i64) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let role = AdminRole::find_by_id(rb, &id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "角色不存在".to_string())?;

        let event = RoleDeletedEvent { id, name: role.name.clone() };

        RoleDomainService::delete(id, event).await?;

        Ok(())
    }

    pub async fn get_role_users(role_id: i64) -> Result<Vec<AdminUser>, String> {
        let rb = &CONTEXT.rbatis;
        UserRole::list_by_role(rb, &role_id).await.map_err(|e| e.to_string())
    }

    pub async fn get_role_permissions(role_id: i64) -> Result<Vec<AdminPermission>, String> {
        let rb = &CONTEXT.rbatis;
        RolePermission::list_by_role(rb, &role_id).await.map_err(|e| e.to_string())
    }

    pub async fn assign_permission(role_id: i64, permission_id: i64) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let role = AdminRole::find_by_id(rb, &role_id).await.ok().flatten();
        let perm = AdminPermission::find_by_id(rb, &permission_id).await.ok().flatten();
        let event = RolePermissionAssignedEvent {
            role_id,
            permission_id,
            role_name: role.as_ref().map(|r| r.name.clone()).unwrap_or_default(),
            resource: perm.as_ref().map(|p| p.resource.clone()).unwrap_or_default(),
            action: perm.as_ref().map(|p| p.action.clone()).unwrap_or_default(),
        };

        RoleDomainService::assign_permission(role_id, permission_id, event).await?;

        Ok(())
    }

    pub async fn revoke_permission(role_id: i64, permission_id: i64) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let role = AdminRole::find_by_id(rb, &role_id).await.ok().flatten();
        let perm = AdminPermission::find_by_id(rb, &permission_id).await.ok().flatten();

        let event = RolePermissionRevokedEvent {
            role_id,
            permission_id,
            role_name: role.map(|r| r.name).unwrap_or_default(),
            resource: perm.as_ref().map(|p| p.resource.clone()).unwrap_or_default(),
            action: perm.as_ref().map(|p| p.action.clone()).unwrap_or_default(),
        };

        RoleDomainService::revoke_permission(role_id, permission_id, event).await?;

        Ok(())
    }
}

// ============================================================================
// PermissionAppService — 权限管理
// ============================================================================

pub struct PermissionAppService;

impl PermissionAppService {
    pub async fn list_all() -> Result<Vec<AdminPermission>, String> {
        let rb = &CONTEXT.rbatis;
        AdminPermission::list_all(rb).await.map_err(|e| e.to_string())
    }

    pub async fn create(input: &serde_json::Value) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let name = input["name"].as_str().unwrap_or("").to_string();
        let resource = input["resource"].as_str().unwrap_or("").to_string();
        let action = input["action"].as_str().unwrap_or("GET").to_string();
        let description = input["description"].as_str().unwrap_or("").to_string();
        let status: i8 = input["status"].as_i64().unwrap_or(1) as i8;

        if name.is_empty() || resource.is_empty() {
            return Err("权限名称和资源不能为空".to_string());
        }

        AdminPermission::insert_permission(rb, &name, &resource, &action, &description, &status)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub async fn get_by_id(id: i64) -> Result<AdminPermission, String> {
        let rb = &CONTEXT.rbatis;
        AdminPermission::find_by_id(rb, &id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "权限不存在".to_string())
    }

    pub async fn update(id: i64, input: &serde_json::Value) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let name = input["name"].as_str().unwrap_or("").to_string();
        let resource = input["resource"].as_str().unwrap_or("").to_string();
        let action = input["action"].as_str().unwrap_or("GET").to_string();
        let description = input["description"].as_str().unwrap_or("").to_string();
        let status: i8 = input["status"].as_i64().unwrap_or(1) as i8;

        AdminPermission::update_by_id(rb, &id, &name, &resource, &action, &description, &status)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub async fn delete(id: i64) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        AdminPermission::delete_by_id(rb, &id).await.map(|_| ()).map_err(|e| e.to_string())
    }
}

// ============================================================================
// DepartmentAppService — 部门管理
// ============================================================================

pub struct DepartmentAppService;

impl DepartmentAppService {
    pub async fn list_all() -> Result<Vec<AdminDepartment>, String> {
        let rb = &CONTEXT.rbatis;
        let mut departments = AdminDepartment::list_all(rb).await.map_err(|e| e.to_string())?;

        // 查询所有部门的成员数量，构建 HashMap 映射
        let counts = UserDepartment::count_members_by_department(rb)
            .await
            .map_err(|e| e.to_string())?;
        let count_map: std::collections::HashMap<i64, i64> = counts
            .into_iter()
            .map(|c| (c.department_id, c.count))
            .collect();

        // 将成员数量填充到每个部门，无成员的部门设为 0
        for dept in &mut departments {
            if let Some(id) = dept.id {
                dept.member_count = Some(*count_map.get(&id).unwrap_or(&0));
            }
        }

        Ok(departments)
    }

    pub async fn create(input: &serde_json::Value) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let name = input["name"].as_str().unwrap_or("").to_string();
        if name.is_empty() {
            return Err("部门名称不能为空".to_string());
        }

        let parent_id = input["parent_id"].as_i64().unwrap_or(0);
        let sort_order = input["sort_order"].as_i64().unwrap_or(0) as i32;
        let description = input["description"].as_str().unwrap_or("").to_string();
        let status: i8 = input["status"].as_i64().unwrap_or(1) as i8;

        AdminDepartment::insert_department(rb, &name, &parent_id, &sort_order, &description, &status)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub async fn get_by_id(id: i64) -> Result<AdminDepartment, String> {
        let rb = &CONTEXT.rbatis;
        AdminDepartment::find_by_id(rb, &id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "部门不存在".to_string())
    }

    pub async fn update(id: i64, input: &serde_json::Value) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let name = input["name"].as_str().unwrap_or("").to_string();
        let parent_id = input["parent_id"].as_i64().unwrap_or(0);
        let sort_order = input["sort_order"].as_i64().unwrap_or(0) as i32;
        let description = input["description"].as_str().unwrap_or("").to_string();
        let status: i8 = input["status"].as_i64().unwrap_or(1) as i8;

        AdminDepartment::update_by_id(rb, &id, &name, &parent_id, &sort_order, &description, &status)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub async fn delete(id: i64) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        AdminDepartment::delete_by_id(rb, &id).await.map(|_| ()).map_err(|e| e.to_string())
    }

    pub async fn move_dept(id: i64, new_parent_id: i64) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        AdminDepartment::move_dept(rb, &id, &new_parent_id).await.map(|_| ()).map_err(|e| e.to_string())
    }
}

// ============================================================================
// UserDepartmentAppService — 用户-部门关联
// ============================================================================

pub struct UserDepartmentAppService;

impl UserDepartmentAppService {
    /// 获取用户的部门列表（返回 AdminDepartment 对象列表）
    pub async fn get_user_departments(user_id: i64) -> Result<Vec<AdminDepartment>, String> {
        let rb = &CONTEXT.rbatis;
        let relations = UserDepartment::list_by_user_id(rb, &user_id)
            .await
            .map_err(|e| e.to_string())?;

        let mut departments = Vec::new();
        for rel in &relations {
            if let Ok(Some(dept)) = AdminDepartment::find_by_id(rb, &rel.department_id).await {
                departments.push(dept);
            }
        }
        Ok(departments)
    }

    /// 分配部门（事务：先删后插）
    pub async fn assign_departments(user_id: i64, department_ids: Vec<i64>) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let tx = rb.acquire_begin().await.map_err(|e| e.to_string())?;

        UserDepartment::delete_by_user_id(&tx, &user_id)
            .await
            .map_err(|e| e.to_string())?;

        if !department_ids.is_empty() {
            UserDepartment::batch_insert(&tx, &user_id, &department_ids)
                .await
                .map_err(|e| e.to_string())?;
        }

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 添加单个用户到部门
    pub async fn add_user_to_department(department_id: i64, user_id: i64) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        UserDepartment::batch_insert(rb, &user_id, &[department_id])
            .await
            .map_err(|e| format!("添加部门成员失败: {}", e))?;
        Ok(())
    }

    /// 从部门移除单个用户
    pub async fn remove_user_from_department(department_id: i64, user_id: i64) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        UserDepartment::remove_user_from_department(rb, &user_id, &department_id)
            .await
            .map_err(|e| format!("移除部门成员失败: {}", e))?;
        Ok(())
    }

    /// 获取部门成员列表（返回 AdminUser 对象列表）
    pub async fn get_department_users(department_id: i64) -> Result<Vec<AdminUser>, String> {
        let rb = &CONTEXT.rbatis;
        let relations = UserDepartment::list_by_department_id(rb, &department_id)
            .await
            .map_err(|e| e.to_string())?;

        let mut users = Vec::new();
        for rel in &relations {
            if let Ok(Some(user)) = AdminUser::find_by_id(rb, &rel.user_id).await {
                users.push(user);
            }
        }
        Ok(users)
    }
}

// ============================================================================
// SyncAppService — 数据同步导出
// ============================================================================

pub struct SyncAppService;

impl SyncAppService {
    /// 查询所有启用状态的用户-角色映射（casbin 'g' 规则格式）
    pub async fn list_active_user_roles() -> Result<Vec<UserRoleMapping>, String> {
        let rb = &CONTEXT.rbatis;
        UserRole::list_active_user_roles(rb)
            .await
            .map_err(|e| e.to_string())
    }
}
