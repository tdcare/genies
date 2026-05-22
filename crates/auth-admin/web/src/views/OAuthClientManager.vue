<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Search, Plus, Refresh, Edit, Delete, Key } from '@element-plus/icons-vue'
import {
  getOAuthClients, createOAuthClient, updateOAuthClient, deleteOAuthClient, regenerateOAuthSecret,
  type OAuthClientRecord, type OAuthClientCreateRequest, type PageData
} from '../api'

const loading = ref(false)
const clientList = ref<OAuthClientRecord[]>([])
const total = ref(0)
const page = ref(1)
const size = ref(10)
const keyword = ref('')

async function loadClients() {
  loading.value = true
  try {
    const data: PageData<OAuthClientRecord> = await getOAuthClients({
      page: page.value, size: size.value,
      keyword: keyword.value || undefined
    })
    clientList.value = data.list
    total.value = data.total
  } catch (e: any) {
    ElMessage.error(e.message || '加载 OAuth 客户端列表失败')
  } finally {
    loading.value = false
  }
}

function handleSearch() { page.value = 1; loadClients() }
function handleSizeChange(val: number) { size.value = val; loadClients() }
function handlePageChange(val: number) { page.value = val; loadClients() }

// ========================================================================
// 创建/编辑弹窗
// ========================================================================

const dialogVisible = ref(false)
const isEdit = ref(false)
const editId = ref<number | null>(null)

// 表单默认值
const defaultForm = () => ({
  client_name: '',
  application_id: 0,
  redirect_uris: [''],
  grant_types: [] as string[],
  scopes: [] as string[],
  token_format: 'jwt',
  access_token_ttl: 3600,
  refresh_token_ttl: 86400,
  require_pkce: 0,
  status: 1
})

const formData = ref(defaultForm())

// 授权类型选项
const grantTypeOptions = [
  { label: 'authorization_code', value: 'authorization_code' },
  { label: 'client_credentials', value: 'client_credentials' },
  { label: 'password', value: 'password' },
  { label: 'refresh_token', value: 'refresh_token' }
]

function openCreate() {
  isEdit.value = false
  editId.value = null
  formData.value = defaultForm()
  dialogVisible.value = true
}

function openEdit(row: OAuthClientRecord) {
  isEdit.value = true
  editId.value = row.id
  // 预填表单
  formData.value = {
    client_name: row.client_name,
    application_id: row.application_id,
    redirect_uris: row.redirect_uris?.length ? row.redirect_uris : [''],
    grant_types: row.grant_types || [],
    scopes: row.scopes || [],
    token_format: row.token_format,
    access_token_ttl: row.access_token_ttl,
    refresh_token_ttl: row.refresh_token_ttl,
    require_pkce: row.require_pkce,
    status: row.status
  }
  dialogVisible.value = true
}

// 重定向 URI 多值管理
function addRedirectUri() { formData.value.redirect_uris.push('') }
function removeRedirectUri(index: number) { formData.value.redirect_uris.splice(index, 1) }
function filteredRedirectUris() {
  return formData.value.redirect_uris.filter(u => u.trim() !== '')
}

async function handleSave() {
  if (!formData.value.client_name) { ElMessage.warning('请输入客户端名称'); return }
  if (formData.value.redirect_uris.filter(u => u.trim()).length === 0) { ElMessage.warning('请至少添加一个重定向 URI'); return }

  loading.value = true
  try {
    if (isEdit.value && editId.value) {
      await updateOAuthClient(editId.value, {
        client_name: formData.value.client_name,
        redirect_uris: filteredRedirectUris(),
        grant_types: formData.value.grant_types,
        scopes: formData.value.scopes,
        token_format: formData.value.token_format,
        access_token_ttl: formData.value.access_token_ttl,
        refresh_token_ttl: formData.value.refresh_token_ttl,
        require_pkce: formData.value.require_pkce,
        status: formData.value.status
      })
      ElMessage.success('更新成功')
      dialogVisible.value = false
      loadClients()
    } else {
      // 创建时，先关闭编辑弹窗，用返回的 secret 打开结果弹窗
      const createReq: OAuthClientCreateRequest = {
        client_name: formData.value.client_name,
        application_id: formData.value.application_id,
        redirect_uris: filteredRedirectUris(),
        grant_types: formData.value.grant_types,
        scopes: formData.value.scopes,
        token_format: formData.value.token_format,
        access_token_ttl: formData.value.access_token_ttl,
        refresh_token_ttl: formData.value.refresh_token_ttl,
        require_pkce: formData.value.require_pkce
      }
      const result = await createOAuthClient(createReq)
      dialogVisible.value = false
      showCreatedSecret(result.client_id, result.client_secret)
      loadClients()
    }
  } catch (e: any) {
    ElMessage.error(e.message || '保存失败')
  } finally {
    loading.value = false
  }
}

// ========================================================================
// 创建成功 - 展示一次性密钥
// ========================================================================

const secretDialogVisible = ref(false)
const secretClientId = ref('')
const secretClientSecret = ref('')

function showCreatedSecret(clientId: string, secret: string) {
  secretClientId.value = clientId
  secretClientSecret.value = secret
  secretDialogVisible.value = true
}

function copySecret() {
  navigator.clipboard.writeText(secretClientSecret.value).then(() => {
    ElMessage.success('密钥已复制到剪贴板')
  }).catch(() => {
    ElMessage.warning('复制失败，请手动复制')
  })
}

// ========================================================================
// 删除
// ========================================================================

async function handleDelete(row: OAuthClientRecord) {
  try {
    await ElMessageBox.confirm(
      `确定要删除客户端 "${row.client_name}" 吗？删除后已签发的令牌将失效。`,
      '确认删除',
      { type: 'warning', confirmButtonText: '确定', cancelButtonText: '取消' }
    )
    await deleteOAuthClient(row.id)
    ElMessage.success('删除成功')
    loadClients()
  } catch { /* 取消 */ }
}

// ========================================================================
// 重新生成密钥
// ========================================================================

async function handleRegenerate(row: OAuthClientRecord) {
  try {
    await ElMessageBox.confirm(
      `重新生成密钥后旧密钥将立即失效，确定要重新生成 "${row.client_name}" 的客户端密钥吗？`,
      '确认重新生成',
      { type: 'warning', confirmButtonText: '确定', cancelButtonText: '取消' }
    )
    const result = await regenerateOAuthSecret(row.id)
    showCreatedSecret(row.client_id, result.client_secret)
  } catch { /* 取消 */ }
}

// ========================================================================
// 字段格式化
// ========================================================================

function formatDate(dateStr?: string): string {
  if (!dateStr) return '-'
  const d = new Date(dateStr)
  if (isNaN(d.getTime())) return dateStr
  return d.toLocaleString('zh-CN')
}

function grantTypeLabel(value: string): string {
  const found = grantTypeOptions.find(o => o.value === value)
  return found?.label || value
}

function ttlLabel(seconds: number): string {
  if (seconds < 60) return `${seconds}s`
  if (seconds < 3600) return `${Math.round(seconds / 60)}min`
  if (seconds < 86400) return `${(seconds / 3600).toFixed(1)}h`
  return `${(seconds / 86400).toFixed(1)}d`
}

onMounted(() => { loadClients() })
</script>

<template>
  <div class="oauth-client-manager">
    <div class="page-header">
      <h2 class="page-title">OAuth 客户端管理</h2>
    </div>

    <!-- 工具栏 -->
    <div class="toolbar">
      <el-input v-model="keyword" placeholder="搜索客户端名称或 Client ID" clearable class="search-input"
        :prefix-icon="Search" @keyup.enter="handleSearch" @clear="handleSearch" />
      <el-button type="primary" :icon="Plus" @click="openCreate">新增客户端</el-button>
      <el-button :icon="Refresh" @click="loadClients">刷新</el-button>
    </div>

    <!-- 表格 -->
    <el-table v-loading="loading" :data="clientList" border stripe class="data-table">
      <el-table-column prop="id" label="ID" width="60" />
      <el-table-column prop="client_name" label="名称" min-width="120" />
      <el-table-column label="Client ID" min-width="180">
        <template #default="{ row }">
          <code style="font-size:12px; color:#409eff;">{{ row.client_id }}</code>
        </template>
      </el-table-column>
      <el-table-column label="授权类型" width="180">
        <template #default="{ row }">
          <el-tag v-for="gt in row.grant_types" :key="gt" size="small" class="grant-tag">
            {{ grantTypeLabel(gt) }}
          </el-tag>
          <span v-if="!row.grant_types?.length" style="color:#909399;">-</span>
        </template>
      </el-table-column>
      <el-table-column label="令牌格式" width="90">
        <template #default="{ row }">
          <el-tag :type="row.token_format === 'jwt' ? 'primary' : 'warning'" size="small" effect="plain">
            {{ row.token_format?.toUpperCase() }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="Access TTL" width="100">
        <template #default="{ row }">{{ ttlLabel(row.access_token_ttl) }}</template>
      </el-table-column>
      <el-table-column label="PKCE" width="80" align="center">
        <template #default="{ row }">
          <el-tag :type="row.require_pkce ? 'success' : 'info'" size="small" effect="plain">
            {{ row.require_pkce ? '需要' : '否' }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="状态" width="80">
        <template #default="{ row }">
          <el-tag :type="row.status === 1 ? 'success' : 'danger'" size="small">{{ row.status === 1 ? '启用' : '禁用' }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="created_at" label="创建时间" width="170">
        <template #default="{ row }">{{ formatDate(row.created_at) }}</template>
      </el-table-column>
      <el-table-column label="操作" width="260" fixed="right">
        <template #default="{ row }">
          <el-button type="warning" link :icon="Edit" @click="openEdit(row)">编辑</el-button>
          <el-button type="primary" link :icon="Key" @click="handleRegenerate(row)">重置密钥</el-button>
          <el-button type="danger" link :icon="Delete" @click="handleDelete(row)">删除</el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- 分页 -->
    <div class="pagination-wrapper">
      <el-pagination
        v-model:current-page="page"
        v-model:page-size="size"
        :total="total"
        :page-sizes="[5, 10, 20, 50]"
        background
        layout="total, sizes, prev, pager, next"
        @size-change="handleSizeChange"
        @current-change="handlePageChange"
      />
    </div>

    <!-- 创建/编辑弹窗 -->
    <el-dialog v-model="dialogVisible" :title="isEdit ? '编辑 OAuth 客户端' : '新增 OAuth 客户端'" width="680px">
      <el-form :model="formData" label-width="130px">
        <el-form-item label="客户端名称" required>
          <el-input v-model="formData.client_name" placeholder="如：Admin Dashboard" />
        </el-form-item>

        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="令牌格式" required>
              <el-radio-group v-model="formData.token_format">
                <el-radio value="jwt">JWT (自包含)</el-radio>
                <el-radio value="opaque">Opaque</el-radio>
              </el-radio-group>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="要求 PKCE" required>
              <el-switch v-model="formData.require_pkce" :active-value="1" :inactive-value="0"
                active-text="是" inactive-text="否" />
            </el-form-item>
          </el-col>
        </el-row>

        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="Access Token TTL (秒)">
              <el-input-number v-model="formData.access_token_ttl" :min="60" :max="86400" :step="300" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="Refresh Token TTL (秒)">
              <el-input-number v-model="formData.refresh_token_ttl" :min="60" :max="2592000" :step="3600" />
            </el-form-item>
          </el-col>
        </el-row>

        <el-form-item label="授权类型" required>
          <el-checkbox-group v-model="formData.grant_types">
            <el-checkbox v-for="opt in grantTypeOptions" :key="opt.value" :value="opt.value" :label="opt.label" />
          </el-checkbox-group>
          <div v-if="formData.grant_types.length === 0" style="color: #f56c6c; font-size: 12px;">请至少选择一种授权类型</div>
        </el-form-item>

        <el-form-item label="Scopes">
          <el-select v-model="formData.scopes" multiple filterable allow-create
            default-first-option placeholder="输入 scope 后回车添加"
            style="width: 100%;">
            <el-option v-for="s in ['read', 'write', 'openid', 'profile', 'email']" :key="s" :value="s" :label="s" />
          </el-select>
        </el-form-item>

        <el-form-item label="重定向 URIs" required>
          <div v-for="(uri, index) in formData.redirect_uris" :key="index" style="display:flex; gap:8px; align-items:center; margin-bottom: 6px;">
            <el-input v-model="formData.redirect_uris[index]" placeholder="如 https://example.com/callback" />
            <el-button v-if="formData.redirect_uris.length > 1" type="danger" :icon="Delete" circle size="small"
              @click="removeRedirectUri(index)" />
          </div>
          <el-button size="small" @click="addRedirectUri">+ 添加</el-button>
        </el-form-item>

        <el-form-item v-if="isEdit" label="状态">
          <el-switch v-model="formData.status" :active-value="1" :inactive-value="0"
            active-text="启用" inactive-text="禁用" />
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="loading" @click="handleSave">确定</el-button>
      </template>
    </el-dialog>

    <!-- 密钥展示弹窗（创建成功 / 重新生成密钥后展示） -->
    <el-dialog v-model="secretDialogVisible" title="客户端密钥" width="520px" :close-on-click-modal="false">
      <el-alert type="warning" :closable="false" show-icon style="margin-bottom: 16px;">
        <template #title>请立即复制并安全保存密钥，关闭后将无法再次查看！</template>
      </el-alert>

      <el-form label-width="80px">
        <el-form-item label="Client ID">
          <div style="width:100%;">
            <code style="font-size:13px; word-break:break-all; color:#409eff;">{{ secretClientId }}</code>
          </div>
        </el-form-item>
        <el-form-item label="Client Secret">
          <div style="display:flex; gap:8px; width:100%; align-items:center;">
            <el-input :model-value="secretClientSecret" readonly type="textarea" :rows="2"
              style="font-family:monospace; font-size:13px;" />
            <el-button type="primary" @click="copySecret">复制</el-button>
          </div>
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button type="primary" @click="secretDialogVisible = false">我已保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
.oauth-client-manager {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.page-header { margin-bottom: 20px; }

.page-title {
  margin: 0;
  font-size: 18px;
  color: #303133;
}

.toolbar {
  display: flex;
  gap: 10px;
  margin-bottom: 16px;
  flex-wrap: wrap;
}

.search-input { width: 300px; }

.data-table { flex: 1; }

.pagination-wrapper {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}

.grant-tag {
  margin-right: 4px;
  margin-bottom: 2px;
}
</style>
