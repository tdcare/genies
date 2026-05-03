<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Refresh, Plus, Delete, Edit, Key as KeyIcon } from '@element-plus/icons-vue'
import {
  getRoles, createRole, updateRole, deleteRole,
  getPermissions, getRolePermissions, assignRolePermission, revokeRolePermission,
  type RoleRecord
} from '../api'

const loading = ref(false)
const roleList = ref<RoleRecord[]>([])

async function loadRoles() {
  loading.value = true
  try {
    roleList.value = await getRoles()
  } catch (e: any) {
    ElMessage.error(e.message || '加载角色列表失败')
  } finally {
    loading.value = false
  }
}

// 创建/编辑弹窗
const dialogVisible = ref(false)
const isEdit = ref(false)
const editId = ref<number | null>(null)
const formData = ref({
  name: '', display_name: '', description: '', parent_id: undefined as number | undefined, status: 1
})

function openCreate() {
  isEdit.value = false
  editId.value = null
  formData.value = { name: '', display_name: '', description: '', parent_id: undefined, status: 1 }
  dialogVisible.value = true
}

function openEdit(row: RoleRecord) {
  isEdit.value = true
  editId.value = row.id
  formData.value = {
    name: row.name,
    display_name: row.display_name,
    description: row.description || '',
    parent_id: row.parent_id,
    status: row.status
  }
  dialogVisible.value = true
}

async function handleSave() {
  if (!formData.value.name || !formData.value.display_name) {
    ElMessage.warning('请填写必要字段')
    return
  }
  loading.value = true
  try {
    const payload: any = { ...formData.value }
    if (!payload.parent_id) payload.parent_id = undefined
    if (isEdit.value && editId.value) {
      await updateRole(editId.value, payload)
      ElMessage.success('更新成功')
    } else {
      await createRole(payload)
      ElMessage.success('创建成功')
    }
    dialogVisible.value = false
    loadRoles()
  } catch (e: any) {
    ElMessage.error(e.message || '保存失败')
  } finally {
    loading.value = false
  }
}

async function handleDelete(row: RoleRecord) {
  try {
    await ElMessageBox.confirm(`确定要删除角色 "${row.display_name}" 吗？`, '确认删除', {
      type: 'warning', confirmButtonText: '确定', cancelButtonText: '取消'
    })
    await deleteRole(row.id)
    ElMessage.success('删除成功')
    loadRoles()
  } catch { /* 取消 */ }
}

// 权限管理弹窗
const permVisible = ref(false)
const permRoleId = ref(0)
const permRoleName = ref('')
const allPermissions = ref<any[]>([])
const rolePermissions = ref<any[]>([])

async function openPermDialog(row: RoleRecord) {
  permRoleId.value = row.id
  permRoleName.value = row.display_name
  permVisible.value = true
  const [perms, rPerms] = await Promise.all([getPermissions(), getRolePermissions(row.id)])
  allPermissions.value = perms
  rolePermissions.value = rPerms
}

function isPermAssigned(permId: number): boolean {
  return rolePermissions.value.some((p: any) => p.permission_id === permId || p.id === permId)
}

async function togglePermission(perm: any) {
  try {
    if (isPermAssigned(perm.id)) {
      await revokeRolePermission(permRoleId.value, perm.id)
      ElMessage.success('已撤销权限')
    } else {
      await assignRolePermission(permRoleId.value, perm.id)
      ElMessage.success('已授予权限')
    }
    rolePermissions.value = await getRolePermissions(permRoleId.value)
  } catch (e: any) {
    ElMessage.error(e.message || '操作失败')
  }
}

onMounted(() => { loadRoles() })
</script>

<template>
  <div class="role-manager">
    <div class="page-header">
      <h2 class="page-title">角色管理</h2>
    </div>

    <!-- 工具栏 -->
    <div class="toolbar">
      <el-button type="primary" :icon="Plus" @click="openCreate">新增角色</el-button>
      <el-button :icon="Refresh" @click="loadRoles">刷新</el-button>
    </div>

    <!-- 表格 -->
    <el-table v-loading="loading" :data="roleList" border stripe class="data-table">
      <el-table-column prop="id" label="ID" width="70" />
      <el-table-column prop="name" label="角色标识" min-width="120" />
      <el-table-column prop="display_name" label="显示名称" min-width="120" />
      <el-table-column prop="description" label="描述" min-width="180" show-overflow-tooltip />
      <el-table-column label="状态" width="90">
        <template #default="{ row }">
          <el-tag :type="row.status === 1 ? 'success' : 'danger'" size="small">
            {{ row.status === 1 ? '启用' : '禁用' }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="260" fixed="right">
        <template #default="{ row }">
          <el-button type="primary" link :icon="Edit" @click="openEdit(row)">编辑</el-button>
          <el-button type="success" link :icon="KeyIcon" @click="openPermDialog(row)">权限</el-button>
          <el-button type="danger" link :icon="Delete" @click="handleDelete(row)">删除</el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- 创建/编辑弹窗 -->
    <el-dialog v-model="dialogVisible" :title="isEdit ? '编辑角色' : '新增角色'" width="500px">
      <el-form :model="formData" label-width="100px">
        <el-form-item label="角色标识" required>
          <el-input v-model="formData.name" placeholder="如: admin, editor" />
        </el-form-item>
        <el-form-item label="显示名称" required>
          <el-input v-model="formData.display_name" placeholder="如: 管理员" />
        </el-form-item>
        <el-form-item label="描述">
          <el-input v-model="formData.description" type="textarea" placeholder="角色描述" />
        </el-form-item>
        <el-form-item label="父角色">
          <el-select v-model="formData.parent_id" placeholder="可选" clearable style="width: 100%">
            <el-option v-for="r in roleList.filter(r => r.id !== editId)" :key="r.id" :label="r.display_name" :value="r.id" />
          </el-select>
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

    <!-- 权限管理弹窗 -->
    <el-dialog v-model="permVisible" :title="`权限授予 - ${permRoleName}`" width="600px">
      <el-table v-if="allPermissions.length > 0" :data="allPermissions" max-height="400" border stripe>
        <el-table-column prop="name" label="权限标识" min-width="120" />
        <el-table-column prop="resource" label="资源" min-width="140" />
        <el-table-column prop="action" label="动作" width="100" />
        <el-table-column label="状态" width="80">
          <template #default="{ row }">
            <el-checkbox
              :model-value="isPermAssigned(row.id)"
              @change="togglePermission(row)"
            />
          </template>
        </el-table-column>
      </el-table>
      <el-empty v-else description="暂无可用权限" />
    </el-dialog>
  </div>
</template>

<style scoped>
.role-manager {
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
