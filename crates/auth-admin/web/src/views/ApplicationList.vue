<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Search, Plus, Refresh, Edit, Delete, Setting } from '@element-plus/icons-vue'
import { getApps, createApp, updateApp, deleteApp, syncAppUserRoles, type AppRecord, type PageData } from '../api'

const router = useRouter()
const loading = ref(false)
const appList = ref<AppRecord[]>([])
const total = ref(0)
const page = ref(1)
const size = ref(10)
const keyword = ref('')

async function loadApps() {
  loading.value = true
  try {
    const data: PageData<AppRecord> = await getApps({
      page: page.value, size: size.value,
      keyword: keyword.value || undefined
    })
    appList.value = data.list
    total.value = data.total
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
    <el-table v-loading="loading" :data="appList" border stripe class="data-table">
      <el-table-column prop="id" label="ID" width="70" />
      <el-table-column prop="app_name" label="应用名称" min-width="120" />
      <el-table-column prop="display_name" label="显示名称" min-width="120" />
      <el-table-column prop="base_url" label="访问地址" min-width="200" />
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
</style>
