<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Search, Plus, Refresh, Edit, Delete, Setting } from '@element-plus/icons-vue'
import { getApps, createApp, updateApp, deleteApp, syncAppUserRoles, getAppInstances, type AppRecord, type InstanceRecord, type PageData } from '../api'

const router = useRouter()
const loading = ref(false)
const appList = ref<AppRecord[]>([])
const total = ref(0)
const page = ref(1)
const size = ref(10)
const keyword = ref('')

// 实例数据：key 为 app id
const instanceMap = ref<Record<number, InstanceRecord[]>>({})
const instanceLoading = ref<Record<number, boolean>>({})

// 相对时间格式化
function timeAgo(dateStr?: string): string {
  if (!dateStr) return '-'
  const now = Date.now()
  const past = new Date(dateStr).getTime()
  if (isNaN(past)) return dateStr
  const diff = Math.max(0, now - past)
  const seconds = Math.floor(diff / 1000)
  if (seconds < 60) return `${seconds}秒前`
  const minutes = Math.floor(seconds / 60)
  if (minutes < 60) return `${minutes}分钟前`
  const hours = Math.floor(minutes / 60)
  if (hours < 24) return `${hours}小时前`
  const days = Math.floor(hours / 24)
  return `${days}天前`
}

// 计算在线实例数
function onlineCount(appId: number): number {
  const list = instanceMap.value[appId]
  if (!list) return 0
  return list.filter(i => i.status === 1).length
}

function totalCount(appId: number): number {
  return instanceMap.value[appId]?.length ?? 0
}

// 实例数颜色
function instanceCountType(appId: number): string {
  const list = instanceMap.value[appId]
  if (!list || list.length === 0) return 'info'
  const online = list.filter(i => i.status === 1).length
  if (online === list.length) return 'success'
  if (online === 0) return 'danger'
  return 'warning'
}

// 展开行时加载实例
async function handleExpandChange(row: AppRecord, expandedRows: AppRecord[]) {
  const isExpanded = expandedRows.some(r => r.id === row.id)
  if (isExpanded && !instanceMap.value[row.id]) {
    await loadInstances(row.id)
  }
}

async function loadInstances(appId: number) {
  instanceLoading.value[appId] = true
  try {
    const data = await getAppInstances(appId)
    instanceMap.value[appId] = data
  } catch (e: any) {
    ElMessage.error(e.message || '加载实例列表失败')
    instanceMap.value[appId] = []
  } finally {
    instanceLoading.value[appId] = false
  }
}

async function loadApps() {
  loading.value = true
  try {
    const data: PageData<AppRecord> = await getApps({
      page: page.value, size: size.value,
      keyword: keyword.value || undefined
    })
    appList.value = data.list
    total.value = data.total
    // 清空旧的实例缓存
    instanceMap.value = {}
  } catch (e: any) {
    ElMessage.error(e.message || '加载应用列表失败')
  } finally {
    loading.value = false
  }
}

function handleSearch() {
  page.value = 1
  loadApps()
}

function handleSizeChange(val: number) {
  size.value = val
  loadApps()
}

function handlePageChange(val: number) {
  page.value = val
  loadApps()
}

// 创建/编辑弹窗
const dialogVisible = ref(false)
const isEdit = ref(false)
const editId = ref<number | null>(null)
const formData = ref({
  app_name: '',
  display_name: '',
  description: '',
  base_url: '',
  status: 1
})

function openCreate() {
  isEdit.value = false
  editId.value = null
  formData.value = { app_name: '', display_name: '', description: '', base_url: '', status: 1 }
  dialogVisible.value = true
}

function openEdit(row: AppRecord) {
  isEdit.value = true
  editId.value = row.id
  formData.value = {
    app_name: row.app_name,
    display_name: row.display_name || '',
    description: row.description || '',
    base_url: row.base_url,
    status: row.status
  }
  dialogVisible.value = true
}

async function handleSave() {
  if (!formData.value.app_name || !formData.value.base_url) {
    ElMessage.warning('请填写应用名称和访问地址')
    return
  }
  loading.value = true
  try {
    if (isEdit.value && editId.value) {
      await updateApp(editId.value, {
        display_name: formData.value.display_name,
        description: formData.value.description,
        base_url: formData.value.base_url,
        status: formData.value.status
      })
      ElMessage.success('更新成功')
    } else {
      await createApp({
        app_name: formData.value.app_name,
        display_name: formData.value.display_name,
        description: formData.value.description,
        base_url: formData.value.base_url
      })
      ElMessage.success('创建成功')
    }
    dialogVisible.value = false
    loadApps()
  } catch (e: any) {
    ElMessage.error(e.message || '保存失败')
  } finally {
    loading.value = false
  }
}

async function handleDelete(row: AppRecord) {
  try {
    await ElMessageBox.confirm(`确定要删除应用 "${row.display_name || row.app_name}" 吗？`, '确认删除', {
      type: 'warning', confirmButtonText: '确定', cancelButtonText: '取消'
    })
    await deleteApp(row.id)
    ElMessage.success('删除成功')
    loadApps()
  } catch { /* 取消 */ }
}

function goPermission(row: AppRecord) {
  router.push(`/apps/${row.id}/permissions`)
}

async function handleSyncRoles(row: AppRecord) {
  try {
    await ElMessageBox.confirm(
      `确定要将用户角色同步到 "${row.display_name || row.app_name}" 吗？`,
      '同步确认',
      { confirmButtonText: '确定', cancelButtonText: '取消', type: 'warning' }
    )
    await syncAppUserRoles(row.id)
    ElMessage.success('用户角色同步成功')
  } catch (e: any) {
    if (e !== 'cancel') {
      ElMessage.error(e.message || '同步失败')
    }
  }
}

onMounted(() => { loadApps() })
</script>

<template>
  <div class="app-list">
    <div class="page-header">
      <h2 class="page-title">应用权限管理</h2>
    </div>

    <!-- 工具栏 -->
    <div class="toolbar">
      <el-input v-model="keyword" placeholder="搜索应用名称" clearable class="search-input"
        :prefix-icon="Search" @keyup.enter="handleSearch" @clear="handleSearch" />
      <el-button type="primary" :icon="Plus" @click="openCreate">新增应用</el-button>
      <el-button :icon="Refresh" @click="loadApps">刷新</el-button>
    </div>

    <!-- 表格 -->
    <el-table v-loading="loading" :data="appList" border stripe class="data-table"
      row-key="id" @expand-change="handleExpandChange">
      <el-table-column type="expand">
        <template #default="{ row }">
          <div class="instance-detail" v-loading="instanceLoading[row.id]">
            <el-table v-if="instanceMap[row.id] && instanceMap[row.id].length > 0"
              :data="instanceMap[row.id]" border size="small" class="instance-table">
              <el-table-column label="实例ID" width="200">
                <template #default="{ row: inst }">
                  <span style="font-family: monospace;">{{ String(inst.instance_id) }}</span>
                </template>
              </el-table-column>
              <el-table-column prop="base_url" label="访问地址" min-width="180" />
              <el-table-column prop="version" label="版本" width="100" />
              <el-table-column label="状态" width="80">
                <template #default="{ row: inst }">
                  <el-tag v-if="inst.status === 1" type="success" size="small">在线</el-tag>
                  <el-tag v-else type="danger" size="small">离线</el-tag>
                </template>
              </el-table-column>
              <el-table-column label="最后心跳" width="130">
                <template #default="{ row: inst }">
                  <el-tooltip :content="inst.last_heartbeat_at || '-'" placement="top">
                    <span>{{ timeAgo(inst.last_heartbeat_at) }}</span>
                  </el-tooltip>
                </template>
              </el-table-column>
              <el-table-column prop="registered_at" label="注册时间" width="170" />
            </el-table>
            <el-empty v-else-if="instanceMap[row.id] && instanceMap[row.id].length === 0"
              description="暂无实例" :image-size="60" />
          </div>
        </template>
      </el-table-column>
      <el-table-column prop="id" label="ID" width="70" />
      <el-table-column prop="app_name" label="应用名称" min-width="120" />
      <el-table-column prop="display_name" label="显示名称" min-width="120" />
      <el-table-column prop="base_url" label="访问地址" min-width="200" />
      <el-table-column label="实例" width="90" align="center">
        <template #default="{ row }">
          <el-tag v-if="instanceMap[row.id]" :type="instanceCountType(row.id)" size="small" effect="plain">
            {{ onlineCount(row.id) }}/{{ totalCount(row.id) }}
          </el-tag>
          <span v-else style="color: #909399; font-size: 12px;">展开查看</span>
        </template>
      </el-table-column>
      <el-table-column label="状态" width="90">
        <template #default="{ row }">
          <el-tag v-if="row.status === 1" type="success">启用</el-tag>
          <el-tag v-else type="danger">禁用</el-tag>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="330" fixed="right">
        <template #default="{ row }">
          <el-button type="primary" link :icon="Setting" @click="goPermission(row)">管理权限</el-button>
          <el-button v-if="row.status === 1" type="success" link @click="handleSyncRoles(row)">同步角色</el-button>
          <el-button type="warning" link :icon="Edit" @click="openEdit(row)">编辑</el-button>
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

    <!-- 新增/编辑弹窗 -->
    <el-dialog v-model="dialogVisible" :title="isEdit ? '编辑应用' : '新增应用'" width="520px">
      <el-form :model="formData" label-width="100px">
        <el-form-item label="应用名称" required>
          <el-input v-model="formData.app_name" :disabled="isEdit" placeholder="唯一应用标识，如 sickbed" />
        </el-form-item>
        <el-form-item label="显示名称">
          <el-input v-model="formData.display_name" placeholder="用于展示的名称" />
        </el-form-item>
        <el-form-item label="访问地址" required>
          <el-input v-model="formData.base_url" placeholder="如 http://localhost:8080" />
        </el-form-item>
        <el-form-item label="描述">
          <el-input v-model="formData.description" type="textarea" :rows="3" placeholder="应用描述" />
        </el-form-item>
        <el-form-item v-if="isEdit" label="状态">
          <el-switch v-model="formData.status" :active-value="1" :inactive-value="0" active-text="启用" inactive-text="禁用" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="loading" @click="handleSave">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
.app-list {
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

.search-input { width: 260px; }

.data-table { flex: 1; }

.pagination-wrapper {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}

.instance-detail {
  padding: 12px 20px;
}

.instance-table {
  width: 100%;
}
</style>
