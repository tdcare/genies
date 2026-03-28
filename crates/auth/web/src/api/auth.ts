import axios from 'axios'

// API 响应格式
export interface ApiResponse<T> {
  code: string
  msg: string
  data: T
}

// 策略记录
export interface PolicyRecord {
  id: number
  ptype: string
  v0: string
  v1: string
  v2: string
  v3?: string
  v4?: string
  v5?: string
}

// 策略 DTO
export interface PolicyDto {
  ptype: string
  v0: string
  v1: string
  v2: string
  v3?: string
  v4?: string
  v5?: string
}

// 模型记录
export interface ModelRecord {
  id: number
  model_name: string
  model_text: string
  description?: string
}

// 模型 DTO
export interface ModelDto {
  model_name: string
  model_text: string
  description?: string
}

// Schema 记录
export interface SchemaRecord {
  id: number
  schema_name: string
  schema_label?: string
  schema_description?: string
  field_name: string
  field_label?: string
  field_type?: string
  field_description?: string
  field_required: boolean
  endpoint_path?: string
  endpoint_label?: string
  endpoint_description?: string
  endpoint_tags?: string
  endpoint_operation_id?: string
  http_method?: string
}

// 创建 axios 实例
const api = axios.create({
  baseURL: '',
  timeout: 30000,
  headers: {
    'Content-Type': 'application/json'
  }
})

// 用于标记是否正在刷新 Token，避免并发刷新
let isRefreshingToken = false
let refreshTokenPromise: Promise<string> | null = null

// 获取临时访问 Token（不需要认证）
export async function getAccessToken(): Promise<{ access_token: string; expires_in: number; token_type: string }> {
  // 注意：这个请求不需要 Authorization header，所以直接用 axios 而不是 api 实例
  const response = await axios.get<ApiResponse<{ access_token: string; expires_in: number; token_type: string }>>('/auth/token')
  if (response.data.code === '0' && response.data.data) {
    return response.data.data
  }
  throw new Error(response.data.msg || '获取 Token 失败')
}

// 请求拦截器：添加 JWT Token，并检测过期自动刷新
api.interceptors.request.use(
  async (config) => {
    let authToken = localStorage.getItem('auth_token')
    const expiresAt = localStorage.getItem('token_expires_at')
    
    // 如果 Token 即将过期（不到 60 秒），尝试刷新
    if (authToken && expiresAt && Date.now() > Number(expiresAt) - 60000) {
      // 避免并发刷新
      if (!isRefreshingToken) {
        isRefreshingToken = true
        refreshTokenPromise = (async () => {
          try {
            const result = await getAccessToken()
            localStorage.setItem('auth_token', result.access_token)
            localStorage.setItem('token_expires_at', String(Date.now() + result.expires_in * 1000))
            return result.access_token
          } finally {
            isRefreshingToken = false
            refreshTokenPromise = null
          }
        })()
      }
      
      if (refreshTokenPromise) {
        try {
          authToken = await refreshTokenPromise
        } catch {
          // 刷新失败，继续使用旧 token
        }
      }
    }
    
    if (authToken) {
      config.headers.Authorization = `Bearer ${authToken}`
    }
    return config
  },
  (error) => {
    return Promise.reject(error)
  }
)

// 响应拦截器：统一处理响应
api.interceptors.response.use(
  (response) => {
    const data = response.data as ApiResponse<unknown>
    if (data.code !== '0') {
      return Promise.reject(new Error(data.msg || '请求失败'))
    }
    return response
  },
  (error) => {
    return Promise.reject(error)
  }
)

// ========== Schema API ==========
export async function getSchemas(): Promise<SchemaRecord[]> {
  const response = await api.get<ApiResponse<SchemaRecord[]>>('/auth/schemas')
  return response.data.data
}

// ========== Model API ==========
export async function getModel(): Promise<ModelRecord> {
  const response = await api.get<ApiResponse<ModelRecord>>('/auth/model')
  return response.data.data
}

export async function updateModel(dto: ModelDto): Promise<void> {
  await api.put('/auth/model', dto)
}

// ========== Policy API ==========
export async function getPolicies(): Promise<PolicyRecord[]> {
  const response = await api.get<ApiResponse<PolicyRecord[]>>('/auth/policies')
  return response.data.data
}

export async function getPoliciesByObject(object: string): Promise<PolicyRecord[]> {
  const response = await api.get<ApiResponse<PolicyRecord[]>>('/auth/policies', { params: { object } })
  return response.data.data
}

export async function addPolicy(dto: PolicyDto): Promise<void> {
  const normalized = { ...dto, v2: dto.v2.toLowerCase() }
  await api.post('/auth/policies', normalized)
}

export async function deletePolicy(id: number): Promise<void> {
  await api.delete(`/auth/policies/${id}`)
}

// ========== Role API ==========
export async function getRoles(): Promise<PolicyRecord[]> {
  const response = await api.get<ApiResponse<PolicyRecord[]>>('/auth/roles')
  return response.data.data
}

export async function addRole(dto: PolicyDto): Promise<void> {
  await api.post('/auth/roles', dto)
}

export async function deleteRole(id: number): Promise<void> {
  await api.delete(`/auth/roles/${id}`)
}

// ========== Group API ==========
export async function getGroups(): Promise<PolicyRecord[]> {
  const response = await api.get<ApiResponse<PolicyRecord[]>>('/auth/groups')
  return response.data.data
}

export async function addGroup(dto: PolicyDto): Promise<void> {
  await api.post('/auth/groups', dto)
}

export async function deleteGroup(id: number): Promise<void> {
  await api.delete(`/auth/groups/${id}`)
}

// ========== Reload API ==========
export async function reloadEnforcer(): Promise<void> {
  await api.post('/auth/reload')
}
