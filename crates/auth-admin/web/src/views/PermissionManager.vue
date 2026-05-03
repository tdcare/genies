<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Refresh, Plus, Delete, Edit } from '@element-plus/icons-vue'
import {
  getPermissions, createPermission, updatePermission, deletePermission,
  type PermissionRecord
} from '../api'

const loading = ref(false)
const permList = ref<PermissionRecord[]>([])

async function loadPermissions() {
  loading.value = true
  try {
    permList.value = await getPermissions()
  } catch (e: any) {
    ElMessage.error(e.message || '加载权限列表失败')
  } finally {
    loading.value = false
  }
}

// 创建/编辑弹窗
const dialogVisible = ref(false)
const isEdit = ref(false)
const editId = ref<number | null>(null)
const formData = ref({
  name: '', resource: '', action: 'GET', description: '', status: 1
})

const actionOptions = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS', '*']

function openCreate() {
  isEdit.value = false
  editId.value = null
  formData.value = { name: '', resource: '', action: 'GET', description: '', status: 1 }
  dialogVisible.value = true
}

function openEdit(row: PermissionRecord) {
  isEdit.value = true
  editId.value = row.id
  formData.value = {
    name: row.name,
    resource: row.resource,
    action: row.action,
    description: row.description || '',
    status: row.status
  }
  dialogVisible.value = true
}

async function handleSave() {
  if (!formData.value.name || !formData.value.resource) {
    ElMessage.warning('请填写必要字段')
    return
  }
  loading.value = true
  try {
    if (isEdit.value && editId.value) {
      await updatePermission(editId.value, formData.value)
      ElMessage.success('更新成功')
    } else {
      await createPermission(formData.value)
      ElMessage.success('创建成功')
    }
    dialogVisible.value = false
    loadPermissions()
  } catch (e: any) {
    ElMessage.error(e.message || '保存失败')
  } finally {
    loading.value = false
  }
}

async function handleDelete(row: PermissionRecord) {
  try {
    await ElMessageBox.confirm(`确定要删除权限 "${row.name}" 吗？`, '确认删除', {
      type: 'warning', confirmButtonText: '确定', cancelButtonText: '取消'
    })
    await deletePermission(row.id)
    ElMessage.success('删除成功')
    loadPermissions()
  } catch { /* 取消 */ }
}

onMounted(() => { loadPermissions() })
</script>

<template>
  <div class="perm-manager">
    <div class="page-header">
      <h2 class="page-title">权限管理</h2>
    </div>

    <!-- 工具栏 -->
    <div class="toolbar">
      <el-button type="primary" :icon="Plus" @click="openCreate">新增权限</el-button>
      <el-button :icon="Refresh" @click="loadPermissions">刷新</el-button>
    </div>

    <!-- 表格 -->
    <el-table v-loading="loading" :data="permList" border stripe class="data-table">
      <el-table-column prop="id" label="ID" width="70" />
      <el-table-column prop="name" label="权限标识" min-width="130" />
      <el-table-column prop="resource" label="资源路径" min-width="200" show-overflow-tooltip />
      <el-table-column prop="action" label="HTTP 方法" width="120">
        <template #default="{ row }">
          <el-tag>{{ row.action }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="description" label="描述" min-width="160" show-overflow-tooltip />
      <el-table-column label="状态" width="90">
        <template #default="{ row }">
          <el-tag :type="row.status === 1 ? 'success' : 'danger'" size="small">
            {{ row.status === 1 ? '启用' : '禁用' }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="180" fixed="right">
        <template #default="{ row }">
          <el-button type="primary" link :icon="Edit" @click="openEdit(row)">编辑</el-button>
          <el-button type="danger" link :icon="Delete" @click="handleDelete(row)">删除</el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- 创建/编辑弹窗 -->
    <el-dialog v-model="dialogVisible" :title="isEdit ? '编辑权限' : '新增权限'" width="500px">
      <el-form :model="formData" label-width="100px">
        <el-form-item label="权限标识" required>
          <el-input v-model="formData.name" placeholder="如: user:read" />
        </el-form-item>
        <el-form-item label="资源路径" required>
          <el-input v-model="formData.resource" placeholder="如: /api/users/*" />
        </el-form-item>
        <el-form-item label="HTTP 方法" required>
          <el-select v-model="formData.action" style="width: 100%">
            <el-option v-for="opt in actionOptions" :key="opt" :label="opt" :value="opt" />
          </el-select>
        </el-form-item>
        <el-form-item label="描述">
          <el-input v-model="formData.description" type="textarea" placeholder="权限描述" />
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
.perm-manager {
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
}

.data-table { flex: 1; }
</style>
