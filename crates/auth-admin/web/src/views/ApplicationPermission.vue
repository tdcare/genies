<script setup lang="ts">
import { ref, onMounted, computed, watch, nextTick } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { ArrowLeft, Plus, Delete, Refresh, Search, Lock } from '@element-plus/icons-vue'
import {
  getApp, getAppSchemas, getAppPolicies, addAppPolicy, deleteAppPolicy,
  getAppRoles, addAppRole, deleteAppRole,
  getAppGroups, addAppGroup, deleteAppGroup,
  reloadAppEnforcer,
  getUsers, getRoles,
  type AppRecord, type UserRecord, type RoleRecord
} from '../api'

// ============================================================================
// 类型定义
// ============================================================================

interface SchemaRecord {
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

interface SchemaGroup {
  schema_name: string
  schema_label?: string
  schema_description?: string
  fields: SchemaRecord[]
  endpoints: Set<string>
}

interface ApiEndpoint {
  http_method: string
  endpoint_path: string
  endpoint_label?: string
  endpoint_description?: string
  endpoint_tags?: string
  endpoint_operation_id?: string
  associated_schemas: string[]
}

interface PolicyRecord {
  id: number
  ptype: string
  v0: string
  v1: string
  v2: string
  v3?: string
  v4?: string
  v5?: string
}

// ============================================================================
// 基础状态
// ============================================================================

const route = useRoute()
const router = useRouter()
const appId = computed(() => Number(route.params.id))

const loading = ref(false)
const app = ref<AppRecord | null>(null)
const activeTab = ref('api')

// ============================================================================
// Tab 1 & 2: Schema / API 数据
// ============================================================================

const schemas = ref<SchemaRecord[]>([])
const policies = ref<PolicyRecord[]>([])
const apiSearchKeyword = ref('')
const searchKeyword = ref('')
const activeNames = ref<string[]>([])
const highlightedApi = ref('')
const apiTableRef = ref<any>(null)

// 权限对话框状态
const permissionDialogVisible = ref(false)
const permissionTargetType = ref<'api' | 'field'>('api')
const permissionTargetName = ref('')
const permissionObjectValue = ref('')
const permissionDefaultAction = ref('')

// 权限对话框表单
const permForm = ref({ subject: '', action: '', effect: 'deny' })
const permPoliciesLoading = ref(false)

// Subject 建议
const adminUsers = ref<UserRecord[]>([])
const adminRoles = ref<RoleRecord[]>([])
const roleSuggestions = ref<{ value: string; label: string }[]>([])

const permActionOptions = computed(() => {
  return permissionTargetType.value === 'api'
    ? ['get', 'post', 'put', 'delete', 'patch', '*']
    : ['read', 'write', '*']
})

const permFilteredPolicies = computed(() => {
  return policies.value.filter(p => p.v1 === permissionObjectValue.value && p.ptype === 'p')
})

function openFieldPermission(schemaName: string, fieldName: string) {
  permissionTargetType.value = 'field'
  permissionTargetName.value = `${schemaName}.${fieldName}`
  permissionObjectValue.value = `${schemaName}.${fieldName}`
  permissionDefaultAction.value = 'read'
  permissionDialogVisible.value = true
}

function openApiPermission(method: string, path: string) {
  permissionTargetType.value = 'api'
  permissionTargetName.value = `${method} ${path}`
  permissionObjectValue.value = path
  permissionDefaultAction.value = method.toLowerCase()
  permissionDialogVisible.value = true
}

watch(() => permissionDialogVisible.value, (v) => {
  if (v) {
    permForm.value = { subject: '', action: permissionDefaultAction.value, effect: 'deny' }
    // 从 auth-admin 用户和角色构建 subject 建议
    const suggestions: { value: string; label: string }[] = []
    for (const u of adminUsers.value) {
      suggestions.push({ value: u.username, label: `${u.username} (用户${u.display_name ? ' - ' + u.display_name : ''})` })
    }
    for (const r of adminRoles.value) {
      suggestions.push({ value: r.name, label: `${r.name} (角色${r.display_name ? ' - ' + r.display_name : ''})` })
    }
    roleSuggestions.value = suggestions
  }
})

function querySubjects(queryString: string, cb: (results: { value: string; label?: string }[]) => void) {
  const results = queryString
    ? roleSuggestions.value.filter(r => r.value.toLowerCase().includes(queryString.toLowerCase()) || r.label.toLowerCase().includes(queryString.toLowerCase()))
    : roleSuggestions.value
  cb(results)
}

async function handleAddPermission() {
  if (!permForm.value.subject.trim()) {
    ElMessage.warning('请输入 Subject')
    return
  }
  try {
    await addAppPolicy(appId.value, {
      ptype: 'p',
      v0: permForm.value.subject.trim(),
      v1: permissionObjectValue.value,
      v2: permForm.value.action,
      v3: permForm.value.effect
    })
    ElMessage.success('添加成功')
    permForm.value.subject = ''
    await loadAllPolicies()
  } catch (e: any) {
    ElMessage.error(e.message || '添加失败')
  }
}

async function handleDeletePermission(policy: PolicyRecord) {
  try {
    await ElMessageBox.confirm(
      `确认删除权限: ${policy.v0} -> ${policy.v2} (${policy.v3})?`,
      '确认删除', { type: 'warning' }
    )
    await deleteAppPolicy(appId.value, policy.id)
    ElMessage.success('删除成功')
    await loadAllPolicies()
  } catch (e: any) {
    if (e !== 'cancel') ElMessage.error(e.message || '删除失败')
  }
}

// 字段权限映射
const fieldPoliciesMap = computed(() => {
  const map = new Map<string, PolicyRecord[]>()
  for (const policy of policies.value) {
    if (policy.ptype !== 'p') continue
    const obj = policy.v1
    if (!obj) continue
    const arr = map.get(obj) || []
    arr.push(policy)
    map.set(obj, arr)
  }
  return map
})

function getFieldPolicies(schemaName: string, fieldName: string): PolicyRecord[] {
  return fieldPoliciesMap.value.get(`${schemaName}.${fieldName}`) || []
}

function getApiPolicies(method: string, path: string): PolicyRecord[] {
  const result: PolicyRecord[] = []
  for (const policy of policies.value) {
    if (policy.ptype !== 'p') continue
    if (policy.v1 === path) {
      if (policy.v2 === '*' || policy.v2.toLowerCase() === method.toLowerCase()) {
        result.push(policy)
      }
    }
  }
  return result
}

function getEffectType(effect: string): 'success' | 'danger' {
  return effect === 'allow' ? 'success' : 'danger'
}

function onTagClick(type: 'field' | 'api', schemaName: string, fieldOrMethod: string, path?: string) {
  if (type === 'field') openFieldPermission(schemaName, fieldOrMethod)
  else if (path) openApiPermission(fieldOrMethod, path)
}

// Schema 分组
const groupedSchemas = computed(() => {
  const groups = new Map<string, SchemaGroup>()
  for (const record of schemas.value) {
    if (!groups.has(record.schema_name)) {
      groups.set(record.schema_name, {
        schema_name: record.schema_name,
        schema_label: record.schema_label,
        schema_description: record.schema_description,
        fields: [],
        endpoints: new Set()
      })
    }
    const group = groups.get(record.schema_name)!
    group.fields.push(record)
    if (record.endpoint_path && record.http_method) {
      group.endpoints.add(`${record.http_method} ${record.endpoint_path}`)
    }
  }
  return Array.from(groups.values())
})

const filteredSchemas = computed(() => {
  if (!searchKeyword.value.trim()) return groupedSchemas.value
  const kw = searchKeyword.value.toLowerCase()
  return groupedSchemas.value.filter(g =>
    g.schema_name.toLowerCase().includes(kw) ||
    g.schema_label?.toLowerCase().includes(kw) ||
    Array.from(g.endpoints).some(ep => ep.toLowerCase().includes(kw))
  )
})

// API 端点
const apiEndpoints = computed(() => {
  const map = new Map<string, ApiEndpoint>()
  for (const record of schemas.value) {
    if (!record.endpoint_path || !record.http_method) continue
    const key = `${record.http_method}|${record.endpoint_path}`
    if (!map.has(key)) {
      map.set(key, {
        http_method: record.http_method,
        endpoint_path: record.endpoint_path,
        endpoint_label: record.endpoint_label,
        endpoint_description: record.endpoint_description,
        endpoint_tags: record.endpoint_tags,
        endpoint_operation_id: record.endpoint_operation_id,
        associated_schemas: []
      })
    }
    const ep = map.get(key)!
    if (!ep.associated_schemas.includes(record.schema_name)) {
      ep.associated_schemas.push(record.schema_name)
    }
  }
  return Array.from(map.values()).sort((a, b) => a.endpoint_path.localeCompare(b.endpoint_path))
})

const filteredApiEndpoints = computed(() => {
  if (!apiSearchKeyword.value.trim()) return apiEndpoints.value
  const kw = apiSearchKeyword.value.toLowerCase()
  return apiEndpoints.value.filter(ep =>
    ep.endpoint_path.toLowerCase().includes(kw) ||
    ep.endpoint_label?.toLowerCase().includes(kw) ||
    ep.endpoint_description?.toLowerCase().includes(kw) ||
    ep.http_method.toLowerCase().includes(kw)
  )
})

function getMethodType(method: string): 'success' | 'primary' | 'warning' | 'danger' | 'info' {
  switch (method.toUpperCase()) {
    case 'GET': return 'success'
    case 'POST': return 'primary'
    case 'PUT': return 'warning'
    case 'DELETE': return 'danger'
    case 'PATCH': return 'warning'
    default: return 'info'
  }
}

async function navigateToSchema(schemaName: string) {
  activeTab.value = 'schema'
  await nextTick()
  if (!activeNames.value.includes(schemaName)) activeNames.value.push(schemaName)
  await nextTick()
  const el = document.getElementById(`schema-${schemaName}`)
  if (el) el.scrollIntoView({ behavior: 'smooth', block: 'start' })
}

async function navigateToApi(endpoint: string) {
  const [method, path] = endpoint.split(' ', 2)
  activeTab.value = 'api'
  apiSearchKeyword.value = ''
  await nextTick()
  highlightedApi.value = `${method}|${path}`
  await nextTick()
  const row = document.querySelector('.api-table .highlighted-row')
  if (row) row.scrollIntoView({ behavior: 'smooth', block: 'center' })
  setTimeout(() => { highlightedApi.value = '' }, 2500)
}

function getApiRowClassName({ row }: { row: ApiEndpoint }) {
  return `${row.http_method}|${row.endpoint_path}` === highlightedApi.value ? 'highlighted-row' : ''
}

function expandAll() { activeNames.value = groupedSchemas.value.map(g => g.schema_name) }
function collapseAll() { activeNames.value = [] }

// ============================================================================
// Tab 3: 策略管理
// ============================================================================

const policySearchKeyword = ref('')
const policyDialogVisible = ref(false)
const policyForm = ref({ ptype: 'p', v0: '', v1: '', v2: 'get', v3: 'allow', v4: '', v5: '' })

const filteredPolicies = computed(() => {
  if (!policySearchKeyword.value.trim()) return policies.value
  const kw = policySearchKeyword.value.toLowerCase()
  return policies.value.filter(p =>
    p.v0?.toLowerCase().includes(kw) ||
    p.v1?.toLowerCase().includes(kw) ||
    p.v2?.toLowerCase().includes(kw)
  )
})

function openAddPolicyDialog() {
  policyForm.value = { ptype: 'p', v0: '', v1: '', v2: 'get', v3: 'allow', v4: '', v5: '' }
  policyDialogVisible.value = true
}

async function handleAddPolicy() {
  if (!policyForm.value.v0 || !policyForm.value.v1) {
    ElMessage.warning('请填写必要字段')
    return
  }
  try {
    await addAppPolicy(appId.value, policyForm.value)
    ElMessage.success('添加策略成功')
    policyDialogVisible.value = false
    await loadAllPolicies()
  } catch (e: any) {
    ElMessage.error(e.message || '添加策略失败')
  }
}

async function handleDeletePolicy(id: number) {
  try {
    await deleteAppPolicy(appId.value, id)
    ElMessage.success('删除策略成功')
    await loadAllPolicies()
  } catch (e: any) {
    ElMessage.error(e.message || '删除策略失败')
  }
}

async function handleReloadEnforcer() {
  try {
    await reloadAppEnforcer(appId.value)
    ElMessage.success('Enforcer 重载成功')
  } catch (e: any) {
    ElMessage.error(e.message || '重载失败')
  }
}

// ============================================================================
// Tab 4: 角色 / 分组
// ============================================================================

const roles = ref<PolicyRecord[]>([])
const groups = ref<PolicyRecord[]>([])
const roleSearchKeyword = ref('')
const groupSearchKeyword = ref('')

const roleDialogVisible = ref(false)
const roleForm = ref({ v0: '', v1: '' })
const groupDialogVisible = ref(false)
const groupForm = ref({ v0: '', v1: '' })

const filteredRoles = computed(() => {
  if (!roleSearchKeyword.value.trim()) return roles.value
  const kw = roleSearchKeyword.value.toLowerCase()
  return roles.value.filter(r =>
    r.v0?.toLowerCase().includes(kw) || r.v1?.toLowerCase().includes(kw)
  )
})

const filteredGroups = computed(() => {
  if (!groupSearchKeyword.value.trim()) return groups.value
  const kw = groupSearchKeyword.value.toLowerCase()
  return groups.value.filter(g =>
    g.v0?.toLowerCase().includes(kw) || g.v1?.toLowerCase().includes(kw)
  )
})

function openAddRole() {
  roleForm.value = { v0: '', v1: '' }
  roleDialogVisible.value = true
}

async function handleAddRole() {
  if (!roleForm.value.v0 || !roleForm.value.v1) {
    ElMessage.warning('请填写用户和角色')
    return
  }
  try {
    await addAppRole(appId.value, { ptype: 'g', ...roleForm.value })
    ElMessage.success('添加角色分配成功')
    roleDialogVisible.value = false
    await loadRoles()
  } catch (e: any) {
    ElMessage.error(e.message || '添加失败')
  }
}

async function handleDeleteRole(id: number) {
  try {
    await deleteAppRole(appId.value, id)
    ElMessage.success('删除角色分配成功')
    await loadRoles()
  } catch (e: any) {
    ElMessage.error(e.message || '删除失败')
  }
}

function openAddGroup() {
  groupForm.value = { v0: '', v1: '' }
  groupDialogVisible.value = true
}

async function handleAddGroup() {
  if (!groupForm.value.v0 || !groupForm.value.v1) {
    ElMessage.warning('请填写子对象和父对象')
    return
  }
  try {
    await addAppGroup(appId.value, { ptype: 'g2', ...groupForm.value })
    ElMessage.success('添加对象分组成功')
    groupDialogVisible.value = false
    await loadGroups()
  } catch (e: any) {
    ElMessage.error(e.message || '添加失败')
  }
}

async function handleDeleteGroup(id: number) {
  try {
    await deleteAppGroup(appId.value, id)
    ElMessage.success('删除分组成功')
    await loadGroups()
  } catch (e: any) {
    ElMessage.error(e.message || '删除失败')
  }
}

// ============================================================================
// 数据加载
// ============================================================================

async function loadAllPolicies() {
  try {
    const data = await getAppPolicies(appId.value)
    policies.value = Array.isArray(data) ? data : []
  } catch (e: any) {
    console.error('加载策略失败', e)
  }
}

async function loadSchemas() {
  loading.value = true
  try {
    const [schemasData] = await Promise.all([
      getAppSchemas(appId.value),
      loadAllPolicies()
    ])
    schemas.value = Array.isArray(schemasData) ? schemasData : []
    if (groupedSchemas.value.length > 0) {
      activeNames.value = [groupedSchemas.value[0].schema_name]
    }
  } catch (e: any) {
    ElMessage.error(e.message || '加载 Schema 列表失败')
  } finally {
    loading.value = false
  }
}

async function loadRoles() {
  try {
    const data = await getAppRoles(appId.value)
    roles.value = Array.isArray(data) ? data : []
  } catch (e: any) {
    ElMessage.error(e.message || '加载角色失败')
  }
}

async function loadGroups() {
  try {
    const data = await getAppGroups(appId.value)
    groups.value = Array.isArray(data) ? data : []
  } catch (e: any) {
    ElMessage.error(e.message || '加载分组失败')
  }
}

function handleTabChange(tab: string) {
  if (tab === 'api' || tab === 'schema') loadSchemas()
  else if (tab === 'policies') loadAllPolicies()
  else if (tab === 'roles') { loadRoles(); loadGroups() }
}

onMounted(async () => {
  try {
    app.value = await getApp(appId.value)
  } catch (e: any) {
    ElMessage.error(e.message || '加载应用信息失败')
  }
  // 并行加载初始数据
  loadSchemas()
  loadRoles()
  loadGroups()
  // 加载 auth-admin 用户和角色（用于 Subject 自动完成）
  loadAdminUsersAndRoles()
})

async function loadAdminUsersAndRoles() {
  try {
    const [usersData, rolesData] = await Promise.all([
      getUsers({ page: 1, size: 9999 }),
      getRoles()
    ])
    adminUsers.value = usersData?.list ?? []
    adminRoles.value = rolesData ?? []
  } catch (e: any) {
    console.error('加载用户/角色数据失败', e)
  }
}
</script>

<template>
  <div class="app-permission" v-loading="loading">
    <!-- 面包屑导航 -->
    <div class="page-header">
      <el-breadcrumb separator="/">
        <el-breadcrumb-item>
          <el-button type="primary" link :icon="ArrowLeft" @click="router.push('/apps')">应用列表</el-button>
        </el-breadcrumb-item>
        <el-breadcrumb-item>{{ app?.display_name || app?.app_name || '...' }}</el-breadcrumb-item>
        <el-breadcrumb-item>权限管理</el-breadcrumb-item>
      </el-breadcrumb>
    </div>

    <!-- 主 Tabs -->
    <el-tabs v-model="activeTab" class="main-tabs" @tab-change="handleTabChange">

      <!-- ============ Tab: API 列表 ============ -->
      <el-tab-pane label="API 列表" name="api">
        <div class="toolbar">
          <el-input v-model="apiSearchKeyword" placeholder="按路径、方法或描述搜索" clearable class="search-input" :prefix-icon="Search" />
          <el-button :icon="Refresh" @click="loadSchemas">刷新</el-button>
        </div>

        <div v-if="filteredApiEndpoints.length === 0 && !loading" class="empty-state">
          <el-empty description="暂无 API 数据" />
        </div>

        <el-table v-else ref="apiTableRef" :data="filteredApiEndpoints" border stripe class="api-table" :row-class-name="getApiRowClassName">
          <el-table-column prop="http_method" label="方法" width="100" align="center">
            <template #default="{ row }">
              <el-tag :type="getMethodType(row.http_method)" size="default" class="method-tag">{{ row.http_method }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="endpoint_path" label="路径" min-width="250">
            <template #default="{ row }">
              <code class="endpoint-path">{{ row.endpoint_path }}</code>
            </template>
          </el-table-column>
          <el-table-column label="标签/描述" min-width="200">
            <template #default="{ row }">
              <div v-if="row.endpoint_label" class="endpoint-info">
                <span class="endpoint-label-text">{{ row.endpoint_label }}</span>
              </div>
              <div v-if="row.endpoint_description" class="endpoint-desc-text">{{ row.endpoint_description }}</div>
              <span v-if="!row.endpoint_label && !row.endpoint_description" class="no-desc">-</span>
            </template>
          </el-table-column>
          <el-table-column label="关联对象" min-width="200">
            <template #default="{ row }">
              <div class="schema-links">
                <el-tag v-for="s in row.associated_schemas" :key="s" type="info" size="small" class="schema-link" @click="navigateToSchema(s)">{{ s }}</el-tag>
              </div>
            </template>
          </el-table-column>
          <el-table-column label="权限" min-width="200">
            <template #default="{ row }">
              <div class="permission-cell">
                <template v-if="getApiPolicies(row.http_method, row.endpoint_path).length > 0">
                  <el-tag v-for="p in getApiPolicies(row.http_method, row.endpoint_path)" :key="p.id" :type="getEffectType(p.v3 || 'deny')" size="small" class="permission-tag" @click="onTagClick('api', '', row.http_method, row.endpoint_path)">
                    {{ p.v0 }}:{{ p.v3 || 'deny' }}
                  </el-tag>
                </template>
                <span v-else class="no-permission">无限制</span>
                <el-button type="primary" :icon="Lock" link size="small" class="set-btn" @click="openApiPermission(row.http_method, row.endpoint_path)">+设置</el-button>
              </div>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <!-- ============ Tab: 对象 (Schema) ============ -->
      <el-tab-pane label="对象" name="schema">
        <div class="toolbar">
          <el-input v-model="searchKeyword" placeholder="按 Schema 名称或端点搜索" clearable class="search-input" :prefix-icon="Search" />
          <el-button @click="expandAll">全部展开</el-button>
          <el-button @click="collapseAll">全部收起</el-button>
          <el-button :icon="Refresh" @click="loadSchemas">刷新</el-button>
        </div>

        <div v-if="filteredSchemas.length === 0 && !loading" class="empty-state">
          <el-empty description="暂无 Schema 数据" />
        </div>

        <el-collapse v-model="activeNames" class="schema-collapse">
          <el-collapse-item v-for="group in filteredSchemas" :key="group.schema_name" :name="group.schema_name" :id="`schema-${group.schema_name}`">
            <template #title>
              <div class="collapse-title">
                <span class="schema-name-label">{{ group.schema_name }}</span>
                <el-tag v-if="group.schema_label" type="info" size="small" class="schema-label-tag">{{ group.schema_label }}</el-tag>
                <el-tooltip v-if="group.schema_description" :content="group.schema_description" placement="top" :disabled="(group.schema_description?.length || 0) <= 30">
                  <span class="schema-description">— {{ (group.schema_description?.length || 0) > 30 ? group.schema_description!.slice(0, 30) + '...' : group.schema_description }}</span>
                </el-tooltip>
                <span class="field-count">{{ group.fields.length }} 个字段</span>
              </div>
            </template>
            <div class="schema-content">
              <div v-if="group.schema_description" class="schema-desc-block"><strong>描述：</strong>{{ group.schema_description }}</div>
              <div v-if="group.endpoints.size > 0" class="endpoints-section">
                <strong>关联端点：</strong>
                <div class="endpoint-tags">
                  <span v-for="ep in Array.from(group.endpoints)" :key="ep" class="endpoint-link" @click.stop="navigateToApi(ep)">{{ ep }}</span>
                </div>
              </div>
              <el-table :data="group.fields" border size="small" class="fields-table">
                <el-table-column prop="field_name" label="字段名" width="150" />
                <el-table-column prop="field_type" label="类型" width="100">
                  <template #default="{ row }"><el-tag size="small">{{ row.field_type || '-' }}</el-tag></template>
                </el-table-column>
                <el-table-column prop="field_label" label="标签" width="120" />
                <el-table-column prop="field_description" label="描述" min-width="200" />
                <el-table-column prop="field_required" label="必填" width="80" align="center">
                  <template #default="{ row }">
                    <el-tag v-if="row.field_required" type="danger" size="small">是</el-tag>
                    <el-tag v-else type="info" size="small">否</el-tag>
                  </template>
                </el-table-column>
                <el-table-column label="权限" min-width="200">
                  <template #default="{ row }">
                    <div class="permission-cell">
                      <template v-if="getFieldPolicies(group.schema_name, row.field_name).length > 0">
                        <el-tag v-for="p in getFieldPolicies(group.schema_name, row.field_name)" :key="p.id" :type="getEffectType(p.v3 || 'deny')" size="small" class="permission-tag" @click="onTagClick('field', group.schema_name, row.field_name)">
                          {{ p.v0 }}:{{ p.v3 || 'deny' }}
                        </el-tag>
                      </template>
                      <span v-else class="no-permission">无限制</span>
                      <el-button type="primary" :icon="Lock" link size="small" class="set-btn" @click="openFieldPermission(group.schema_name, row.field_name)">+设置</el-button>
                    </div>
                  </template>
                </el-table-column>
              </el-table>
            </div>
          </el-collapse-item>
        </el-collapse>
      </el-tab-pane>

      <!-- ============ Tab: 策略管理 ============ -->
      <el-tab-pane label="策略规则" name="policies">
        <div class="toolbar">
          <el-input v-model="policySearchKeyword" placeholder="按 Subject/Object/Action 搜索" clearable class="search-input" :prefix-icon="Search" />
          <el-button type="primary" :icon="Plus" @click="openAddPolicyDialog">新增策略</el-button>
          <el-button type="warning" :icon="Refresh" @click="handleReloadEnforcer">重载 Enforcer</el-button>
          <el-button :icon="Refresh" @click="loadAllPolicies">刷新</el-button>
        </div>
        <el-table :data="filteredPolicies" border stripe class="policy-table">
          <el-table-column prop="id" label="ID" width="80" />
          <el-table-column prop="ptype" label="ptype" width="80" />
          <el-table-column prop="v0" label="v0 (subject)" min-width="120" />
          <el-table-column prop="v1" label="v1 (object)" min-width="180" />
          <el-table-column prop="v2" label="v2 (action)" width="100" />
          <el-table-column prop="v3" label="v3 (effect)" width="100">
            <template #default="{ row }">
              <el-tag v-if="row.v3" :type="getEffectType(row.v3)" size="small">{{ row.v3 }}</el-tag>
              <span v-else>-</span>
            </template>
          </el-table-column>
          <el-table-column prop="v4" label="v4" width="100" />
          <el-table-column prop="v5" label="v5" width="100" />
          <el-table-column label="操作" width="100" fixed="right">
            <template #default="{ row }">
              <el-popconfirm title="确定要删除这条策略吗？" confirm-button-text="确定" cancel-button-text="取消" @confirm="handleDeletePolicy(row.id)">
                <template #reference>
                  <el-button type="danger" :icon="Delete" link>删除</el-button>
                </template>
              </el-popconfirm>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <!-- ============ Tab: 角色 / 分组 ============ -->
      <el-tab-pane label="角色 / 分组" name="roles">
        <div class="roles-groups-container">
          <!-- 左侧：角色分配 -->
          <div class="half-panel">
            <div class="panel-header">
              <h4>角色分配（g 规则）</h4>
              <div class="panel-actions">
                <el-input v-model="roleSearchKeyword" placeholder="搜索" clearable size="small" style="width: 140px;" :prefix-icon="Search" />
                <el-button type="primary" :icon="Plus" size="small" @click="openAddRole">添加</el-button>
                <el-button :icon="Refresh" size="small" @click="loadRoles">刷新</el-button>
              </div>
            </div>
            <el-table :data="filteredRoles" border stripe size="small">
              <el-table-column prop="id" label="ID" width="60" />
              <el-table-column prop="v0" label="用户" min-width="140">
                <template #default="{ row }"><el-tag>{{ row.v0 }}</el-tag></template>
              </el-table-column>
              <el-table-column prop="v1" label="角色" min-width="140">
                <template #default="{ row }"><el-tag type="success">{{ row.v1 }}</el-tag></template>
              </el-table-column>
              <el-table-column label="操作" width="80" fixed="right">
                <template #default="{ row }">
                  <el-popconfirm title="确定要删除这条角色分配吗？" confirm-button-text="确定" cancel-button-text="取消" @confirm="handleDeleteRole(row.id)">
                    <template #reference>
                      <el-button type="danger" :icon="Delete" link size="small">删除</el-button>
                    </template>
                  </el-popconfirm>
                </template>
              </el-table-column>
            </el-table>
          </div>
          <!-- 右侧：对象分组 -->
          <div class="half-panel">
            <div class="panel-header">
              <h4>对象分组（g2 规则）</h4>
              <div class="panel-actions">
                <el-input v-model="groupSearchKeyword" placeholder="搜索" clearable size="small" style="width: 140px;" :prefix-icon="Search" />
                <el-button type="primary" :icon="Plus" size="small" @click="openAddGroup">添加</el-button>
                <el-button :icon="Refresh" size="small" @click="loadGroups">刷新</el-button>
              </div>
            </div>
            <el-table :data="filteredGroups" border stripe size="small">
              <el-table-column prop="id" label="ID" width="60" />
              <el-table-column prop="v0" label="资源路径" min-width="200">
                <template #default="{ row }"><el-tag type="info">{{ row.v0 }}</el-tag></template>
              </el-table-column>
              <el-table-column prop="v1" label="分组名" min-width="150">
                <template #default="{ row }"><el-tag type="warning">{{ row.v1 }}</el-tag></template>
              </el-table-column>
              <el-table-column label="操作" width="80" fixed="right">
                <template #default="{ row }">
                  <el-popconfirm title="确定要删除这条分组映射吗？" confirm-button-text="确定" cancel-button-text="取消" @confirm="handleDeleteGroup(row.id)">
                    <template #reference>
                      <el-button type="danger" :icon="Delete" link size="small">删除</el-button>
                    </template>
                  </el-popconfirm>
                </template>
              </el-table-column>
            </el-table>
          </div>
        </div>
      </el-tab-pane>
    </el-tabs>

    <!-- ============ 权限设置对话框（内联） ============ -->
    <el-dialog v-model="permissionDialogVisible" :title="'权限设置 - ' + permissionTargetName" width="650px" :close-on-click-modal="false" class="permission-dialog">
      <div class="target-info">
        <el-tag :type="permissionTargetType === 'api' ? 'primary' : 'warning'" size="default">{{ permissionTargetType === 'api' ? 'API' : '字段' }}</el-tag>
        <code class="target-value">{{ permissionObjectValue }}</code>
      </div>
      <div class="add-form">
        <el-form :inline="true" @submit.prevent="handleAddPermission">
          <el-form-item label="Subject">
            <el-autocomplete v-model="permForm.subject" :fetch-suggestions="querySubjects" placeholder="输入或选择角色/用户" clearable style="width: 200px">
              <template #default="{ item }">
                <span>{{ item.label || item.value }}</span>
              </template>
            </el-autocomplete>
          </el-form-item>
          <el-form-item label="Action">
            <el-select v-model="permForm.action" style="width: 100px">
              <el-option v-for="a in permActionOptions" :key="a" :label="a" :value="a" />
            </el-select>
          </el-form-item>
          <el-form-item label="Effect">
            <el-select v-model="permForm.effect" style="width: 90px">
              <el-option label="deny" value="deny" />
              <el-option label="allow" value="allow" />
            </el-select>
          </el-form-item>
          <el-form-item>
            <el-button type="primary" :icon="Plus" @click="handleAddPermission">添加</el-button>
          </el-form-item>
        </el-form>
      </div>
      <div class="policy-list">
        <h4>现有权限规则</h4>
        <el-table :data="permFilteredPolicies" border size="small" empty-text="暂无权限规则">
          <el-table-column prop="v0" label="Subject" min-width="120" />
          <el-table-column prop="v2" label="Action" width="100" align="center">
            <template #default="{ row }"><code>{{ row.v2 }}</code></template>
          </el-table-column>
          <el-table-column prop="v3" label="Effect" width="100" align="center">
            <template #default="{ row }">
              <el-tag :type="getEffectType(row.v3 || 'deny')" size="small">{{ row.v3 }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column label="操作" width="80" align="center">
            <template #default="{ row }">
              <el-button type="danger" :icon="Delete" link size="small" @click="handleDeletePermission(row)">删除</el-button>
            </template>
          </el-table-column>
        </el-table>
      </div>
      <template #footer>
        <el-button @click="permissionDialogVisible = false">关闭</el-button>
      </template>
    </el-dialog>

    <!-- 添加策略弹窗 -->
    <el-dialog v-model="policyDialogVisible" title="新增策略" width="500px">
      <el-form :model="policyForm" label-width="100px">
        <el-form-item label="类型 (ptype)">
          <el-select v-model="policyForm.ptype" style="width: 100%">
            <el-option label="p (策略)" value="p" />
            <el-option label="p2 (策略2)" value="p2" />
          </el-select>
        </el-form-item>
        <el-form-item label="Subject (v0)" required>
          <el-input v-model="policyForm.v0" placeholder="例如：admin, user, role:admin" />
        </el-form-item>
        <el-form-item label="Object (v1)" required>
          <el-input v-model="policyForm.v1" placeholder="例如：/api/users, /api/orders/*" />
        </el-form-item>
        <el-form-item label="Action (v2)" required>
          <el-select v-model="policyForm.v2" style="width: 100%">
            <el-option v-for="opt in ['get','post','put','delete','patch','*']" :key="opt" :label="opt" :value="opt" />
          </el-select>
        </el-form-item>
        <el-form-item label="Effect (v3)">
          <el-select v-model="policyForm.v3" style="width: 100%" allow-create filterable>
            <el-option v-for="opt in ['allow','deny']" :key="opt" :label="opt" :value="opt" />
          </el-select>
        </el-form-item>
        <el-form-item label="v4">
          <el-input v-model="policyForm.v4" placeholder="可选字段" />
        </el-form-item>
        <el-form-item label="v5">
          <el-input v-model="policyForm.v5" placeholder="可选字段" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="policyDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleAddPolicy">确定</el-button>
      </template>
    </el-dialog>

    <!-- 添加角色分配弹窗 -->
    <el-dialog v-model="roleDialogVisible" title="添加角色分配" width="450px">
      <el-form :model="roleForm" label-width="80px">
        <el-form-item label="用户名" required>
          <el-input v-model="roleForm.v0" placeholder="输入用户名或用户标识" />
        </el-form-item>
        <el-form-item label="角色名" required>
          <el-input v-model="roleForm.v1" placeholder="输入角色名称，如 admin, editor" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="roleDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleAddRole">确定</el-button>
      </template>
    </el-dialog>

    <!-- 添加对象分组弹窗 -->
    <el-dialog v-model="groupDialogVisible" title="添加对象分组" width="450px">
      <el-form :model="groupForm" label-width="80px">
        <el-form-item label="子对象" required>
          <el-input v-model="groupForm.v0" placeholder="子对象标识（如资源路径）" />
        </el-form-item>
        <el-form-item label="父对象" required>
          <el-input v-model="groupForm.v1" placeholder="父对象标识（如分组名）" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="groupDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleAddGroup">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
.app-permission {
  height: 100%;
  display: flex;
  flex-direction: column;
}
.page-header { margin-bottom: 20px; }

.main-tabs { flex: 1; display: flex; flex-direction: column; }
.main-tabs :deep(.el-tabs__content) { flex: 1; overflow-y: auto; }

.toolbar { display: flex; gap: 10px; margin-bottom: 20px; flex-wrap: wrap; }
.search-input { width: 300px; }

.empty-state { flex: 1; display: flex; align-items: center; justify-content: center; padding: 40px; }

/* API 表格 */
.api-table { margin-top: 0; }
.method-tag { font-weight: 600; font-family: monospace; min-width: 60px; }
.endpoint-path { font-family: monospace; font-size: 13px; color: #606266; background: #f5f7fa; padding: 2px 6px; border-radius: 4px; }
.endpoint-info { margin-bottom: 4px; }
.endpoint-label-text { font-weight: 500; color: #303133; }
.endpoint-desc-text { font-size: 12px; color: #909399; }
.no-desc { color: #c0c4cc; }

.schema-links { display: flex; flex-wrap: wrap; gap: 6px; }
.schema-link { cursor: pointer; transition: all 0.2s; }
.schema-link:hover { background-color: #409eff; color: #fff; border-color: #409eff; }

/* Schema Collapse */
.schema-collapse { flex: 1; overflow-y: auto; }
.collapse-title { display: flex; align-items: center; gap: 10px; }
.schema-name-label { font-weight: 600; color: #409eff; }
.schema-label-tag { margin-left: 8px; }
.schema-description { color: #909399; font-size: 12px; font-style: italic; margin-left: 8px; max-width: 300px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.field-count { color: #909399; font-size: 12px; margin-left: auto; margin-right: 10px; }
.schema-content { padding: 10px 0; }
.schema-desc-block { margin-bottom: 15px; color: #606266; font-size: 14px; }
.endpoints-section { margin-bottom: 15px; }
.endpoint-tags { margin-top: 8px; display: flex; flex-wrap: wrap; gap: 8px; }
.endpoint-link { font-family: monospace; font-size: 12px; color: #409eff; cursor: pointer; padding: 4px 8px; background: #ecf5ff; border-radius: 4px; transition: all 0.2s; }
.endpoint-link:hover { background: #409eff; color: #fff; }
.fields-table { margin-top: 10px; }

/* 权限列 */
.permission-cell { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; }
.permission-tag { cursor: pointer; transition: all 0.2s; }
.permission-tag:hover { transform: scale(1.05); box-shadow: 0 2px 6px rgba(0,0,0,0.15); }
.no-permission { color: #c0c4cc; font-size: 12px; }
.set-btn { margin-left: 4px; padding: 2px 6px; }

/* 高亮行 */
.api-table :deep(.highlighted-row) { background-color: #e6f7ff !important; animation: highlight-fade 2.5s ease-out; }
.api-table :deep(.highlighted-row td) { background-color: #e6f7ff !important; }
@keyframes highlight-fade { 0% { background-color: #bae7ff; } 70% { background-color: #e6f7ff; } 100% { background-color: transparent; } }

/* 策略表 */
.policy-table { flex: 1; }

/* 角色/分组 */
.roles-groups-container { display: flex; gap: 20px; }
.half-panel { flex: 1; min-width: 0; }
.panel-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px; flex-wrap: wrap; gap: 8px; }
.panel-header h4 { margin: 0; font-size: 15px; color: #303133; }
.panel-actions { display: flex; gap: 8px; align-items: center; }

/* 权限对话框 */
.permission-dialog :deep(.el-dialog__body) { padding-top: 15px; }
.target-info { display: flex; align-items: center; gap: 12px; margin-bottom: 20px; padding: 12px; background: #f5f7fa; border-radius: 6px; }
.target-value { font-family: monospace; font-size: 14px; color: #606266; background: #e6e8eb; padding: 4px 8px; border-radius: 4px; }
.add-form { margin-bottom: 20px; padding: 15px; background: #fafafa; border: 1px solid #ebeef5; border-radius: 6px; }
.add-form :deep(.el-form-item) { margin-bottom: 0; margin-right: 12px; }
.add-form :deep(.el-form-item__label) { font-size: 13px; }
.policy-list h4 { margin: 0 0 12px 0; font-size: 14px; color: #303133; }
.policy-list :deep(.el-table) { font-size: 13px; }
</style>
