<script setup lang="ts">
import { ref, computed, onMounted, nextTick } from 'vue'
import { ElMessage } from 'element-plus'
import { Refresh, Search, Lock } from '@element-plus/icons-vue'
import { getSchemas, getPolicies, type SchemaRecord, type PolicyRecord } from '../api/auth'
import PermissionDialog from '../components/PermissionDialog.vue'

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

const loading = ref(false)
const schemas = ref<SchemaRecord[]>([])
const policies = ref<PolicyRecord[]>([])
const searchKeyword = ref('')
const activeNames = ref<string[]>([])
const activeTab = ref('api')
const apiSearchKeyword = ref('')
const highlightedApi = ref<string>('')
const apiTableRef = ref<any>(null)

// 权限对话框状态
const permissionDialogVisible = ref(false)
const permissionTargetType = ref<'api' | 'field'>('api')
const permissionTargetName = ref('')
const permissionObjectValue = ref('')
const permissionDefaultAction = ref('')

// 打开字段权限对话框
function openFieldPermission(schemaName: string, fieldName: string) {
  permissionTargetType.value = 'field'
  permissionTargetName.value = `${schemaName}.${fieldName}`
  permissionObjectValue.value = `${schemaName}.${fieldName}`
  permissionDefaultAction.value = 'read'
  permissionDialogVisible.value = true
}

// 打开 API 权限对话框
function openApiPermission(method: string, path: string) {
  permissionTargetType.value = 'api'
  permissionTargetName.value = `${method} ${path}`
  permissionObjectValue.value = path
  permissionDefaultAction.value = method.toLowerCase()
  permissionDialogVisible.value = true
}

// 权限变更回调 - 刷新策略数据
async function onPermissionChanged() {
  await loadPolicies()
}

// 字段权限映射: key = "SchemaName.field_name", value = 匹配的策略数组
const fieldPoliciesMap = computed(() => {
  const map = new Map<string, PolicyRecord[]>()
  for (const policy of policies.value) {
    if (policy.ptype !== 'p') continue
    const objectValue = policy.v1
    if (!objectValue) continue
    const existing = map.get(objectValue) || []
    existing.push(policy)
    map.set(objectValue, existing)
  }
  return map
})

// 获取字段的权限列表
function getFieldPolicies(schemaName: string, fieldName: string): PolicyRecord[] {
  const key = `${schemaName}.${fieldName}`
  return fieldPoliciesMap.value.get(key) || []
}

// 获取 API 的权限列表 - 匹配 v1 (endpoint_path) 和 v2 (http_method)
function getApiPolicies(method: string, path: string): PolicyRecord[] {
  const result: PolicyRecord[] = []
  for (const policy of policies.value) {
    if (policy.ptype !== 'p') continue
    // v1 是 object (path), v2 是 action (method)
    if (policy.v1 === path) {
      // 检查 method 匹配 (v2 可能是 * 或具体 method)
      if (policy.v2 === '*' || policy.v2.toLowerCase() === method.toLowerCase()) {
        result.push(policy)
      }
    }
  }
  return result
}

// 获取 effect 标签类型
function getEffectType(effect: string): 'success' | 'danger' {
  return effect === 'allow' ? 'success' : 'danger'
}

// 点击权限标签时打开弹框
function onTagClick(type: 'field' | 'api', schemaName: string, fieldOrMethod: string, path?: string) {
  if (type === 'field') {
    openFieldPermission(schemaName, fieldOrMethod)
  } else if (path) {
    openApiPermission(fieldOrMethod, path)
  }
}

const groupedSchemas = computed(() => {
  const groups: Map<string, SchemaGroup> = new Map()
  
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
  if (!searchKeyword.value.trim()) {
    return groupedSchemas.value
  }
  const keyword = searchKeyword.value.toLowerCase()
  return groupedSchemas.value.filter(group => 
    group.schema_name.toLowerCase().includes(keyword) ||
    group.schema_label?.toLowerCase().includes(keyword) ||
    Array.from(group.endpoints).some(ep => ep.toLowerCase().includes(keyword))
  )
})

// API 列表分组
const apiEndpoints = computed(() => {
  const endpointMap: Map<string, ApiEndpoint> = new Map()
  
  for (const record of schemas.value) {
    if (!record.endpoint_path || !record.http_method) continue
    
    const key = `${record.http_method}|${record.endpoint_path}`
    if (!endpointMap.has(key)) {
      endpointMap.set(key, {
        http_method: record.http_method,
        endpoint_path: record.endpoint_path,
        endpoint_label: record.endpoint_label,
        endpoint_description: record.endpoint_description,
        endpoint_tags: record.endpoint_tags,
        endpoint_operation_id: record.endpoint_operation_id,
        associated_schemas: []
      })
    }
    const endpoint = endpointMap.get(key)!
    if (!endpoint.associated_schemas.includes(record.schema_name)) {
      endpoint.associated_schemas.push(record.schema_name)
    }
  }
  
  // 按路径排序
  return Array.from(endpointMap.values()).sort((a, b) => 
    a.endpoint_path.localeCompare(b.endpoint_path)
  )
})

const filteredApiEndpoints = computed(() => {
  if (!apiSearchKeyword.value.trim()) {
    return apiEndpoints.value
  }
  const keyword = apiSearchKeyword.value.toLowerCase()
  return apiEndpoints.value.filter(ep => 
    ep.endpoint_path.toLowerCase().includes(keyword) ||
    ep.endpoint_label?.toLowerCase().includes(keyword) ||
    ep.endpoint_description?.toLowerCase().includes(keyword) ||
    ep.http_method.toLowerCase().includes(keyword)
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
  // 展开并滚动到指定 schema
  if (!activeNames.value.includes(schemaName)) {
    activeNames.value.push(schemaName)
  }
  await nextTick()
  // 滚动到目标元素
  const element = document.getElementById(`schema-${schemaName}`)
  if (element) {
    element.scrollIntoView({ behavior: 'smooth', block: 'start' })
  }
}

async function navigateToApi(endpoint: string) {
  // endpoint 格式: "GET /api/users"
  const [method, path] = endpoint.split(' ', 2)
  const apiKey = `${method}|${path}`
  
  // 切换到 API 列表 tab
  activeTab.value = 'api'
  // 清空搜索以显示完整列表
  apiSearchKeyword.value = ''
  
  await nextTick()
  
  // 设置高亮
  highlightedApi.value = apiKey
  
  await nextTick()
  
  // 滚动到高亮行
  const highlightedRow = document.querySelector('.api-table .highlighted-row')
  if (highlightedRow) {
    highlightedRow.scrollIntoView({ behavior: 'smooth', block: 'center' })
  }
  
  // 2.5秒后清除高亮
  setTimeout(() => {
    highlightedApi.value = ''
  }, 2500)
}

function getApiRowClassName({ row }: { row: ApiEndpoint }) {
  const key = `${row.http_method}|${row.endpoint_path}`
  return key === highlightedApi.value ? 'highlighted-row' : ''
}

async function loadPolicies() {
  try {
    policies.value = await getPolicies()
  } catch (error: any) {
    console.error('加载策略失败', error)
  }
}

async function loadSchemas() {
  loading.value = true
  try {
    // 并行加载 schemas 和 policies
    const [schemasData] = await Promise.all([
      getSchemas(),
      loadPolicies()
    ])
    schemas.value = schemasData
    // 默认展开第一个
    if (groupedSchemas.value.length > 0) {
      activeNames.value = [groupedSchemas.value[0].schema_name]
    }
  } catch (error: any) {
    ElMessage.error(error.message || '加载 Schema 列表失败')
  } finally {
    loading.value = false
  }
}

function expandAll() {
  activeNames.value = groupedSchemas.value.map(g => g.schema_name)
}

function collapseAll() {
  activeNames.value = []
}

onMounted(() => {
  loadSchemas()
})
</script>

<template>
  <div class="schema-explorer" v-loading="loading">
    <div class="page-header">
      <h2 class="page-title">权限设置</h2>
      <p class="page-desc">管理 API 和字段的访问权限</p>
    </div>
    
    <el-tabs v-model="activeTab" class="main-tabs">
      <el-tab-pane label="API 列表" name="api">
        <div class="toolbar">
          <el-input
            v-model="apiSearchKeyword"
            placeholder="按路径、方法或描述搜索"
            clearable
            class="search-input"
            :prefix-icon="Search"
          />
          <el-button :icon="Refresh" @click="loadSchemas">刷新</el-button>
        </div>

        <div v-if="filteredApiEndpoints.length === 0 && !loading" class="empty-state">
          <el-empty description="暂无 API 数据" />
        </div>

        <el-table 
          v-else
          ref="apiTableRef"
          :data="filteredApiEndpoints" 
          border 
          stripe 
          class="api-table"
          :row-class-name="getApiRowClassName"
        >
          <el-table-column prop="http_method" label="方法" width="100" align="center">
            <template #default="{ row }">
              <el-tag 
                :type="getMethodType(row.http_method)" 
                size="default"
                class="method-tag"
              >
                {{ row.http_method }}
              </el-tag>
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
              <div v-if="row.endpoint_description" class="endpoint-desc-text">
                {{ row.endpoint_description }}
              </div>
              <span v-if="!row.endpoint_label && !row.endpoint_description" class="no-desc">-</span>
            </template>
          </el-table-column>
          <el-table-column label="关联对象" min-width="200">
            <template #default="{ row }">
              <div class="schema-links">
                <el-tag
                  v-for="schema in row.associated_schemas"
                  :key="schema"
                  type="info"
                  size="small"
                  class="schema-link"
                  @click="navigateToSchema(schema)"
                >
                  {{ schema }}
                </el-tag>
              </div>
            </template>
          </el-table-column>
          <el-table-column label="权限" min-width="200">
            <template #default="{ row }">
              <div class="permission-cell">
                <template v-if="getApiPolicies(row.http_method, row.endpoint_path).length > 0">
                  <el-tag
                    v-for="policy in getApiPolicies(row.http_method, row.endpoint_path)"
                    :key="policy.id"
                    :type="getEffectType(policy.v3 || 'deny')"
                    size="small"
                    class="permission-tag"
                    @click="onTagClick('api', '', row.http_method, row.endpoint_path)"
                  >
                    {{ policy.v0 }}:{{ policy.v3 || 'deny' }}
                  </el-tag>
                </template>
                <span v-else class="no-permission">无限制</span>
                <el-button
                  type="primary"
                  :icon="Lock"
                  link
                  size="small"
                  class="set-btn"
                  @click="openApiPermission(row.http_method, row.endpoint_path)"
                >
                  +设置
                </el-button>
              </div>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <el-tab-pane label="对象" name="schema">
        <div class="toolbar">
          <el-input
            v-model="searchKeyword"
            placeholder="按 Schema 名称或端点路径搜索"
            clearable
            class="search-input"
            :prefix-icon="Search"
          />
          <el-button @click="expandAll">全部展开</el-button>
          <el-button @click="collapseAll">全部收起</el-button>
          <el-button :icon="Refresh" @click="loadSchemas">刷新</el-button>
        </div>

        <div v-if="filteredSchemas.length === 0 && !loading" class="empty-state">
          <el-empty description="暂无 Schema 数据" />
        </div>

        <el-collapse v-model="activeNames" class="schema-collapse">
          <el-collapse-item
            v-for="group in filteredSchemas"
            :key="group.schema_name"
            :name="group.schema_name"
            :id="`schema-${group.schema_name}`"
          >
            <template #title>
              <div class="collapse-title">
                <span class="schema-name">{{ group.schema_name }}</span>
                <el-tag v-if="group.schema_label" type="info" size="small" class="schema-label">
                  {{ group.schema_label }}
                </el-tag>
                <el-tooltip
                  v-if="group.schema_description"
                  :content="group.schema_description"
                  placement="top"
                  :disabled="group.schema_description.length <= 30"
                >
                  <span class="schema-description">— {{ group.schema_description.length > 30 ? group.schema_description.slice(0, 30) + '...' : group.schema_description }}</span>
                </el-tooltip>
                <span class="field-count">{{ group.fields.length }} 个字段</span>
              </div>
            </template>

            <div class="schema-content">
              <div v-if="group.schema_description" class="schema-desc">
                <strong>描述：</strong>{{ group.schema_description }}
              </div>

              <div v-if="group.endpoints.size > 0" class="endpoints-section">
                <strong>关联端点：</strong>
                <div class="endpoint-tags">
                  <span
                    v-for="endpoint in Array.from(group.endpoints)"
                    :key="endpoint"
                    class="endpoint-link"
                    @click.stop="navigateToApi(endpoint)"
                  >
                    {{ endpoint }}
                  </span>
                </div>
              </div>

              <el-table :data="group.fields" border size="small" class="fields-table">
                <el-table-column prop="field_name" label="字段名" width="150" />
                <el-table-column prop="field_type" label="类型" width="100">
                  <template #default="{ row }">
                    <el-tag size="small">{{ row.field_type || '-' }}</el-tag>
                  </template>
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
                        <el-tag
                          v-for="policy in getFieldPolicies(group.schema_name, row.field_name)"
                          :key="policy.id"
                          :type="getEffectType(policy.v3 || 'deny')"
                          size="small"
                          class="permission-tag"
                          @click="onTagClick('field', group.schema_name, row.field_name)"
                        >
                          {{ policy.v0 }}:{{ policy.v3 || 'deny' }}
                        </el-tag>
                      </template>
                      <span v-else class="no-permission">无限制</span>
                      <el-button
                        type="primary"
                        :icon="Lock"
                        link
                        size="small"
                        class="set-btn"
                        @click="openFieldPermission(group.schema_name, row.field_name)"
                      >
                        +设置
                      </el-button>
                    </div>
                  </template>
                </el-table-column>
              </el-table>
            </div>
          </el-collapse-item>
        </el-collapse>
      </el-tab-pane>
    </el-tabs>

    <!-- 权限设置对话框 -->
    <PermissionDialog
      v-model:visible="permissionDialogVisible"
      :target-type="permissionTargetType"
      :target-name="permissionTargetName"
      :object-value="permissionObjectValue"
      :default-action="permissionDefaultAction"
      @changed="onPermissionChanged"
    />
  </div>
</template>

<style scoped>
.schema-explorer {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.page-header {
  margin-bottom: 20px;
}

.page-title {
  margin: 0 0 8px 0;
  font-size: 18px;
  color: #303133;
}

.page-desc {
  margin: 0;
  font-size: 14px;
  color: #909399;
}

.main-tabs {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.main-tabs :deep(.el-tabs__content) {
  flex: 1;
  overflow-y: auto;
}

.toolbar {
  display: flex;
  gap: 10px;
  margin-bottom: 20px;
}

.search-input {
  width: 300px;
}

.empty-state {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 40px;
}

.schema-collapse {
  flex: 1;
  overflow-y: auto;
}

.collapse-title {
  display: flex;
  align-items: center;
  gap: 10px;
}

.schema-name {
  font-weight: 600;
  color: #409eff;
}

.schema-label {
  margin-left: 8px;
}

.schema-description {
  color: #909399;
  font-size: 12px;
  font-style: italic;
  margin-left: 8px;
  max-width: 300px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.field-count {
  color: #909399;
  font-size: 12px;
  margin-left: auto;
  margin-right: 10px;
}

.schema-content {
  padding: 10px 0;
}

.schema-desc {
  margin-bottom: 15px;
  color: #606266;
  font-size: 14px;
}

.endpoints-section {
  margin-bottom: 15px;
}

.endpoint-tags {
  margin-top: 8px;
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.endpoint-tag {
  font-family: monospace;
}

.endpoint-link {
  font-family: monospace;
  font-size: 12px;
  color: #409eff;
  cursor: pointer;
  padding: 4px 8px;
  background: #ecf5ff;
  border-radius: 4px;
  transition: all 0.2s;
}

.endpoint-link:hover {
  background: #409eff;
  color: #fff;
  text-decoration: underline;
}

.fields-table {
  margin-top: 10px;
}

/* 权限单元格样式 */
.permission-cell {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
}

.permission-tag {
  cursor: pointer;
  transition: all 0.2s;
}

.permission-tag:hover {
  transform: scale(1.05);
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.15);
}

.no-permission {
  color: #c0c4cc;
  font-size: 12px;
}

.set-btn {
  margin-left: 4px;
  padding: 2px 6px;
}

/* API 列表样式 */
.api-table {
  margin-top: 0;
}

.method-tag {
  font-weight: 600;
  font-family: monospace;
  min-width: 60px;
}

.endpoint-path {
  font-family: monospace;
  font-size: 13px;
  color: #606266;
  background: #f5f7fa;
  padding: 2px 6px;
  border-radius: 4px;
}

.endpoint-info {
  margin-bottom: 4px;
}

.endpoint-label-text {
  font-weight: 500;
  color: #303133;
}

.endpoint-desc-text {
  font-size: 12px;
  color: #909399;
}

.no-desc {
  color: #c0c4cc;
}

.schema-links {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.schema-link {
  cursor: pointer;
  transition: all 0.2s;
}

.schema-link:hover {
  background-color: #409eff;
  color: #fff;
  border-color: #409eff;
}

/* 高亮行样式 */
.api-table :deep(.highlighted-row) {
  background-color: #e6f7ff !important;
  animation: highlight-fade 2.5s ease-out;
}

.api-table :deep(.highlighted-row td) {
  background-color: #e6f7ff !important;
}

@keyframes highlight-fade {
  0% {
    background-color: #bae7ff;
  }
  70% {
    background-color: #e6f7ff;
  }
  100% {
    background-color: transparent;
  }
}
</style>
