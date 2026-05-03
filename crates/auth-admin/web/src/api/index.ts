import axios, { type InternalAxiosRequestConfig, type AxiosResponse, type AxiosError } from 'axios'
import { ElMessage } from 'element-plus'

// ============================================================================
// 通用响应格式
// ============================================================================

export interface ApiResponse<T> {
  code: string
  msg: string
  data: T
}

export interface PageData<T> {
  total: number
  page: number
  size: number
  list: T[]
}

// ============================================================================
// 类型定义
// ============================================================================

export interface UserRecord {
  id: number
  username: string
  display_name: string
  email?: string
  phone?: string
  avatar?: string
  status: number
  last_login_at?: string
  created_at?: string
  updated_at?: string
}

export interface RoleRecord {
  id: number
  name: string
  display_name: string
  description?: string
  parent_id?: number
  status: number
  created_at?: string
  updated_at?: string
}

export interface PermissionRecord {
  id: number
  name: string
  resource: string
  action: string
  description?: string
  status: number
  created_at?: string
  updated_at?: string
}

export interface DepartmentRecord {
  id: number
  name: string
  parent_id?: number
  sort_order?: number
  description?: string
  status: number
  created_at?: string
  updated_at?: string
}

export interface LoginResponse {
  access_token: string
  token_type: string
  expires_in: number
  username: string
  display_name: string
}

// ============================================================================
// Axios 实例
// ============================================================================

function getApiBaseUrl(): string {
  const path = window.location.pathname
  const idx = path.indexOf('/auth-admin/ui')
  return idx > 0 ? path.substring(0, idx) : ''
}

const api = axios.create({
  baseURL: getApiBaseUrl(),
  timeout: 30000,
  headers: { 'Content-Type': 'application/json' }
})

let isRefreshingToken = false
let refreshTokenPromise: Promise<string> | null = null

// 请求拦截器：添加 JWT Token，过期自动刷新
api.interceptors.request.use(
  async (config: InternalAxiosRequestConfig) => {
    let authToken = localStorage.getItem('admin_token')
    const expiresAt = localStorage.getItem('admin_token_expires_at')

    if (authToken && expiresAt && Date.now() > Number(expiresAt) - 60000) {
      if (!isRefreshingToken) {
        isRefreshingToken = true
        refreshTokenPromise = (async () => {
          try {
            const result = await api.post<ApiResponse<{ access_token: string; expires_in: number }>>('/auth-admin/refresh')
            if ((result.data.code === 'SUCCESS' || result.data.code === '0') && result.data.data) {
              localStorage.setItem('admin_token', result.data.data.access_token)
              localStorage.setItem('admin_token_expires_at', String(Date.now() + result.data.data.expires_in * 1000))
              return result.data.data.access_token
            }
            throw new Error('刷新失败')
          } finally {
            isRefreshingToken = false
            refreshTokenPromise = null
          }
        })()
      }

      if (refreshTokenPromise) {
        try { authToken = await refreshTokenPromise } catch { /* 继续用旧 token */ }
      }
    }

    if (authToken) {
      config.headers.Authorization = `Bearer ${authToken}`
    }
    return config
  },
  (error: AxiosError) => Promise.reject(error)
)

// 响应拦截器：统一处理 code 并跳转登录
api.interceptors.response.use(
  (response: AxiosResponse) => {
    const data = response.data as ApiResponse<unknown>
    const isSuccess = data.code === 'SUCCESS' || data.code === '0'
    if (!isSuccess) {
      if (data.code === '-1' && response.config.url?.includes('/auth-admin/refresh')) {
        // 刷新失败，跳转登录
        localStorage.removeItem('admin_token')
        localStorage.removeItem('admin_token_expires_at')
        localStorage.removeItem('admin_user')
        window.location.href = getApiBaseUrl() + '/auth-admin/ui/#/login'
      }
      return Promise.reject(new Error(data.msg || '请求失败'))
    }
    return response
  },
  (error: AxiosError) => {
    if (error.response?.status === 401) {
      localStorage.removeItem('admin_token')
      localStorage.removeItem('admin_token_expires_at')
      localStorage.removeItem('admin_user')
      ElMessage.error('登录已过期，请重新登录')
      window.location.href = getApiBaseUrl() + '/auth-admin/ui/#/login'
    }
    return Promise.reject(error)
  }
)

// ============================================================================
// Auth API
// ============================================================================

export async function login(username: string, password: string): Promise<LoginResponse> {
  const response = await api.post<ApiResponse<LoginResponse>>('/auth-admin/login', { username, password })
  const data = response.data.data
  localStorage.setItem('admin_token', data.access_token)
  localStorage.setItem('admin_token_expires_at', String(Date.now() + data.expires_in * 1000))
  localStorage.setItem('admin_user', JSON.stringify({ username: data.username, display_name: data.display_name }))
  return data
}

export async function logout(): Promise<void> {
  try { await api.post('/auth-admin/logout') } catch { /* ignore */ }
  localStorage.removeItem('admin_token')
  localStorage.removeItem('admin_token_expires_at')
  localStorage.removeItem('admin_user')
}

export async function getMe(): Promise<any> {
  const response = await api.get<ApiResponse<any>>('/auth-admin/me')
  return response.data.data
}

export async function changePassword(oldPassword: string, newPassword: string): Promise<void> {
  await api.put('/auth-admin/me/password', { old_password: oldPassword, new_password: newPassword })
}

// ============================================================================
// Users API
// ============================================================================

export async function getUsers(params: { page?: number; size?: number; keyword?: string }): Promise<PageData<UserRecord>> {
  const response = await api.get<ApiResponse<PageData<UserRecord>>>('/auth-admin/users', { params })
  return response.data.data
}

export async function createUser(data: Partial<UserRecord> & { password?: string }): Promise<{ id: number }> {
  const response = await api.post<ApiResponse<{ id: number }>>('/auth-admin/users', data)
  return response.data.data
}

export async function updateUser(id: number, data: Partial<UserRecord>): Promise<void> {
  await api.put(`/auth-admin/users/${id}`, data)
}

export async function deleteUser(id: number): Promise<void> {
  await api.delete(`/auth-admin/users/${id}`)
}

export async function batchDeleteUsers(ids: number[]): Promise<void> {
  await api.post('/auth-admin/users/batch-delete', { ids })
}

export async function updateUserStatus(id: number, status: number): Promise<void> {
  await api.put(`/auth-admin/users/${id}/status`, { status })
}

export async function resetUserPassword(id: number, password: string): Promise<void> {
  await api.put(`/auth-admin/users/${id}/reset-password`, { password })
}

export async function getUserRoles(userId: number): Promise<any[]> {
  const response = await api.get<ApiResponse<any[]>>(`/auth-admin/users/${userId}/roles`)
  return response.data.data
}

export async function assignUserRole(userId: number, roleId: number): Promise<void> {
  await api.post(`/auth-admin/users/${userId}/roles`, { role_id: roleId })
}

export async function revokeUserRole(userId: number, roleId: number): Promise<void> {
  await api.delete(`/auth-admin/users/${userId}/roles/${roleId}`)
}

export async function getUserPermissions(userId: number): Promise<any[]> {
  const response = await api.get<ApiResponse<any[]>>(`/auth-admin/users/${userId}/permissions`)
  return response.data.data
}

// ============================================================================
// Roles API
// ============================================================================

export async function getRoles(): Promise<RoleRecord[]> {
  const response = await api.get<ApiResponse<RoleRecord[]>>('/auth-admin/roles')
  return response.data.data
}

export async function createRole(data: Partial<RoleRecord>): Promise<{ id: number }> {
  const response = await api.post<ApiResponse<{ id: number }>>('/auth-admin/roles', data)
  return response.data.data
}

export async function updateRole(id: number, data: Partial<RoleRecord>): Promise<void> {
  await api.put(`/auth-admin/roles/${id}`, data)
}

export async function deleteRole(id: number): Promise<void> {
  await api.delete(`/auth-admin/roles/${id}`)
}

export async function getRoleUsers(roleId: number): Promise<any[]> {
  const response = await api.get<ApiResponse<any[]>>(`/auth-admin/roles/${roleId}/users`)
  return response.data.data
}

export async function getRolePermissions(roleId: number): Promise<any[]> {
  const response = await api.get<ApiResponse<any[]>>(`/auth-admin/roles/${roleId}/permissions`)
  return response.data.data
}

export async function assignRolePermission(roleId: number, permissionId: number): Promise<void> {
  await api.post(`/auth-admin/roles/${roleId}/permissions`, { permission_id: permissionId })
}

export async function revokeRolePermission(roleId: number, permissionId: number): Promise<void> {
  await api.delete(`/auth-admin/roles/${roleId}/permissions/${permissionId}`)
}

// ============================================================================
// Permissions API
// ============================================================================

export async function getPermissions(): Promise<PermissionRecord[]> {
  const response = await api.get<ApiResponse<PermissionRecord[]>>('/auth-admin/permissions')
  return response.data.data
}

export async function createPermission(data: Partial<PermissionRecord>): Promise<void> {
  await api.post('/auth-admin/permissions', data)
}

export async function updatePermission(id: number, data: Partial<PermissionRecord>): Promise<void> {
  await api.put(`/auth-admin/permissions/${id}`, data)
}

export async function deletePermission(id: number): Promise<void> {
  await api.delete(`/auth-admin/permissions/${id}`)
}

// ============================================================================
// Departments API
// ============================================================================

export async function getDepartments(): Promise<DepartmentRecord[]> {
  const response = await api.get<ApiResponse<DepartmentRecord[]>>('/auth-admin/departments')
  return response.data.data
}

export async function createDepartment(data: Partial<DepartmentRecord>): Promise<void> {
  await api.post('/auth-admin/departments', data)
}

export async function updateDepartment(id: number, data: Partial<DepartmentRecord>): Promise<void> {
  await api.put(`/auth-admin/departments/${id}`, data)
}

export async function deleteDepartment(id: number): Promise<void> {
  await api.delete(`/auth-admin/departments/${id}`)
}

export async function moveDepartment(id: number, parentId: number): Promise<void> {
  await api.put(`/auth-admin/departments/${id}/move/${parentId}`)
}

export async function getDepartmentUsers(departmentId: number): Promise<any[]> {
  const response = await api.get<ApiResponse<any[]>>(`/auth-admin/departments/${departmentId}/users`)
  return response.data.data
}

export async function getUserDepartments(userId: number): Promise<DepartmentRecord[]> {
  const response = await api.get<ApiResponse<DepartmentRecord[]>>(`/auth-admin/users/${userId}/departments`)
  return response.data.data
}

export async function assignUserDepartments(userId: number, departmentIds: number[]): Promise<void> {
  await api.post(`/auth-admin/users/${userId}/departments`, departmentIds)
}

// ============================================================================
// Applications API（应用管理）
// ============================================================================

export interface AppRecord {
  id: number
  app_name: string
  display_name?: string
  description?: string
  base_url: string
  status: number
  created_at?: string
  updated_at?: string
}

// 应用 CRUD
export const getApps = (params?: { page?: number; size?: number; keyword?: string }) =>
  api.get<ApiResponse<PageData<AppRecord>>>('/auth-admin/apps', { params }).then(r => r.data.data)

export const getApp = (id: number) =>
  api.get<ApiResponse<AppRecord>>(`/auth-admin/apps/${id}`).then(r => r.data.data)

export const createApp = (data: { app_name: string; display_name?: string; description?: string; base_url: string }) =>
  api.post('/auth-admin/apps', data)

export const updateApp = (id: number, data: { display_name?: string; description?: string; base_url?: string; status?: number }) =>
  api.put(`/auth-admin/apps/${id}`, data)

export const deleteApp = (id: number) =>
  api.delete(`/auth-admin/apps/${id}`)

// 应用权限代理 API
export const getAppSchemas = (appId: number) =>
  api.get<ApiResponse<any>>(`/auth-admin/apps/${appId}/schemas`).then(r => r.data.data)

export const getAppPolicies = (appId: number, params?: { object?: string; subject?: string }) =>
  api.get<ApiResponse<any>>(`/auth-admin/apps/${appId}/policies`, { params }).then(r => r.data.data)

export const addAppPolicy = (appId: number, data: any) =>
  api.post(`/auth-admin/apps/${appId}/policies`, data)

export const deleteAppPolicy = (appId: number, policyId: number) =>
  api.delete(`/auth-admin/apps/${appId}/policies/${policyId}`)

export const getAppRoles = (appId: number) =>
  api.get<ApiResponse<any>>(`/auth-admin/apps/${appId}/roles`).then(r => r.data.data)

export const addAppRole = (appId: number, data: any) =>
  api.post(`/auth-admin/apps/${appId}/roles`, data)

export const getAppGroups = (appId: number) =>
  api.get<ApiResponse<any>>(`/auth-admin/apps/${appId}/groups`).then(r => r.data.data)

export const addAppGroup = (appId: number, data: any) =>
  api.post(`/auth-admin/apps/${appId}/groups`, data)

export const deleteAppRole = (appId: number, roleId: number) =>
  api.delete(`/auth-admin/apps/${appId}/roles/${roleId}`)

export const deleteAppGroup = (appId: number, groupId: number) =>
  api.delete(`/auth-admin/apps/${appId}/groups/${groupId}`)

export const reloadAppEnforcer = (appId: number) =>
  api.post(`/auth-admin/apps/${appId}/reload`)

export const syncAppUserRoles = (appId: number) =>
  api.post(`/auth-admin/apps/${appId}/sync-user-roles`)
