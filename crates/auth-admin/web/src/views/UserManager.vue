<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Search, Refresh, Plus, Delete, Edit, Key as KeyIcon, UserFilled, OfficeBuilding } from '@element-plus/icons-vue'
import type { ElTree } from 'element-plus'
import {
  getUsers, createUser, updateUser, deleteUser, batchDeleteUsers,
  updateUserStatus, resetUserPassword, getUserRoles, assignUserRole, revokeUserRole,
  getRoles, getDepartments, getUserDepartments, assignUserDepartments,
  type UserRecord, type PageData, type DepartmentRecord
} from '../api'

const loading = ref(false)
const userList = ref<UserRecord[]>([])
const total = ref(0)
const page = ref(1)
const size = ref(10)
const keyword = ref('')
const selectedIds = ref<number[]>([])

async function loadUsers() {
  loading.value = true
  try {
    const data: PageData<UserRecord> = await getUsers({
      page: page.value, size: size.value,
      keyword: keyword.value || undefined
    })
    userList.value = data.list
    total.value = data.total
  } catch (e: any) {
    ElMessage.error(e.message || '加载用户列表失败')
  } finally {
    loading.value = false
  }
}

function handleSearch() {
  page.value = 1
  loadUsers()
}

function handleSizeChange(val: number) {
  size.value = val
  loadUsers()
}

function handlePageChange(val: number) {
  page.value = val
  loadUsers()
}

// 创建/编辑弹窗
const dialogVisible = ref(false)
const isEdit = ref(false)
const editId = ref<number | null>(null)
const formData = ref({
  username: '', password: '', display_name: '', email: '', phone: '', status: 1
})

function openCreate() {
  isEdit.value = false
  editId.value = null
  formData.value = { username: '', password: '123456', display_name: '', email: '', phone: '', status: 1 }
  dialogVisible.value = true
}

function openEdit(row: UserRecord) {
  isEdit.value = true
  editId.value = row.id
  formData.value = {
    username: row.username,
    password: '',
    display_name: row.display_name,
    email: row.email || '',
    phone: row.phone || '',
    status: row.status
  }
  dialogVisible.value = true
}

async function handleSave() {
  if (!formData.value.username || !formData.value.display_name) {
    ElMessage.warning('请填写必要字段')
    return
  }
  loading.value = true
  try {
    if (isEdit.value && editId.value) {
      await updateUser(editId.value, {
        username: formData.value.username,
        display_name: formData.value.display_name,
        email: formData.value.email,
        phone: formData.value.phone,
        status: formData.value.status
      })
      ElMessage.success('更新成功')
    } else {
      await createUser({
        username: formData.value.username,
        password: formData.value.password || '123456',
        display_name: formData.value.display_name,
        email: formData.value.email,
        phone: formData.value.phone
      })
      ElMessage.success('创建成功')
    }
    dialogVisible.value = false
    loadUsers()
  } catch (e: any) {
    ElMessage.error(e.message || '保存失败')
  } finally {
    loading.value = false
  }
}

async function handleDelete(row: UserRecord) {
  try {
    await ElMessageBox.confirm(`确定要删除用户 "${row.display_name}" 吗？`, '确认删除', {
      type: 'warning', confirmButtonText: '确定', cancelButtonText: '取消'
    })
    await deleteUser(row.id)
    ElMessage.success('删除成功')
    loadUsers()
  } catch { /* 取消 */ }
}

async function handleBatchDelete() {
  if (selectedIds.value.length === 0) {
    ElMessage.warning('请先选择用户')
    return
  }
  try {
    await ElMessageBox.confirm(`确定要删除选中的 ${selectedIds.value.length} 个用户吗？`, '批量删除', {
      type: 'warning', confirmButtonText: '确定', cancelButtonText: '取消'
    })
    await batchDeleteUsers(selectedIds.value)
    ElMessage.success('删除成功')
    selectedIds.value = []
    loadUsers()
  } catch { /* 取消 */ }
}

function handleSelectionChange(rows: UserRecord[]) {
  selectedIds.value = rows.map(r => r.id)
}

async function toggleStatus(row: UserRecord) {
  const newStatus = row.status === 1 ? 0 : 1
  try {
    await updateUserStatus(row.id, newStatus)
    row.status = newStatus
    ElMessage.success(newStatus === 1 ? '已启用' : '已禁用')
  } catch (e: any) {
    ElMessage.error(e.message || '操作失败')
  }
}

// 重置密码弹窗
const pwdVisible = ref(false)
const pwdUserId = ref(0)
const pwdUserName = ref('')
const newPassword = ref('123456')

function openResetPassword(row: UserRecord) {
  pwdUserId.value = row.id
  pwdUserName.value = row.display_name
  newPassword.value = '123456'
  pwdVisible.value = true
}

async function handleResetPassword() {
  try {
    await resetUserPassword(pwdUserId.value, newPassword.value)
    ElMessage.success('密码已重置')
    pwdVisible.value = false
  } catch (e: any) {
    ElMessage.error(e.message || '重置失败')
  }
}

// 角色管理弹窗
const roleVisible = ref(false)
const roleUserId = ref(0)
const roleUserName = ref('')
const allRoles = ref<any[]>([])
const userRoles = ref<any[]>([])

async function openRoleDialog(row: UserRecord) {
  roleUserId.value = row.id
  roleUserName.value = row.display_name
  roleVisible.value = true
  const [roles, uRoles] = await Promise.all([getRoles(), getUserRoles(row.id)])
  allRoles.value = roles
  userRoles.value = uRoles
}

function isRoleAssigned(roleId: number): boolean {
  return userRoles.value.some((r: any) => r.role_id === roleId || r.id === roleId)
}

async function toggleRole(role: any) {
  try {
    if (isRoleAssigned(role.id)) {
      await revokeUserRole(roleUserId.value, role.id)
      ElMessage.success('已移除角色')
    } else {
      await assignUserRole(roleUserId.value, role.id)
      ElMessage.success('已分配角色')
    }
    userRoles.value = await getUserRoles(roleUserId.value)
  } catch (e: any) {
    ElMessage.error(e.message || '操作失败')
  }
}

onMounted(() => { loadUsers() })

// 部门分配弹窗
const deptVisible = ref(false)
const deptLoading = ref(false)
const deptUserId = ref(0)
const deptUserName = ref('')
const deptTreeData = ref<any[]>([])
const deptCheckedKeys = ref<number[]>([])
const deptTreeRef = ref<InstanceType<typeof ElTree>>()

function buildDeptTree(list: DepartmentRecord[], parentId: number = 0): any[] {
  return list
    .filter(d => (d.parent_id ?? 0) === parentId)
    .map(d => {
      const children = buildDeptTree(list, d.id)
      return children.length > 0 ? { ...d, children } : { ...d }
    })
}

async function openDeptDialog(row: UserRecord) {
  deptUserId.value = row.id
  deptUserName.value = row.display_name
  deptVisible.value = true
  deptLoading.value = true
  try {
    const [allDepts, userDepts] = await Promise.all([
      getDepartments(),
      getUserDepartments(row.id)
    ])
    deptTreeData.value = buildDeptTree(allDepts)
    deptCheckedKeys.value = userDepts.map(d => d.id)
  } catch (e: any) {
    ElMessage.error(e.message || '加载部门信息失败')
  } finally {
    deptLoading.value = false
  }
}

async function handleSaveDepts() {
  const tree = deptTreeRef.value
  if (!tree) return
  const checkedIds = tree.getCheckedKeys(false) as number[]
  deptLoading.value = true
  try {
    await assignUserDepartments(deptUserId.value, checkedIds)
    ElMessage.success('部门分配成功')
    deptVisible.value = false
  } catch (e: any) {
    ElMessage.error(e.message || '部门分配失败')
  } finally {
    deptLoading.value = false
  }
}
</script>

<template>
  <div class="user-manager">
    <div class="page-header">
      <h2 class="page-title">用户管理</h2>
    </div>

    <!-- 工具栏 -->
    <div class="toolbar">
      <el-input v-model="keyword" placeholder="搜索用户名/显示名称" clearable class="search-input"
        :prefix-icon="Search" @keyup.enter="handleSearch" @clear="handleSearch" />
      <el-button type="primary" :icon="Plus" @click="openCreate">新增</el-button>
      <el-button type="danger" :icon="Delete" @click="handleBatchDelete" :disabled="selectedIds.length === 0">
        批量删除 {{ selectedIds.length > 0 ? `(${selectedIds.length})` : '' }}
      </el-button>
      <el-button :icon="Refresh" @click="loadUsers">刷新</el-button>
    </div>

    <!-- 表格 -->
    <el-table v-loading="loading" :data="userList" border stripe class="data-table"
      @selection-change="handleSelectionChange">
      <el-table-column type="selection" width="50" />
      <el-table-column prop="id" label="ID" width="70" />
      <el-table-column prop="username" label="用户名" min-width="120" />
      <el-table-column prop="display_name" label="显示名称" min-width="120" />
      <el-table-column prop="email" label="邮箱" min-width="160" />
      <el-table-column prop="phone" label="手机" width="130" />
      <el-table-column label="状态" width="90">
        <template #default="{ row }">
          <el-switch :model-value="row.status === 1" @change="toggleStatus(row)" />
        </template>
      </el-table-column>
      <el-table-column label="操作" width="360" fixed="right">
        <template #default="{ row }">
          <el-button type="primary" link :icon="Edit" @click="openEdit(row)">编辑</el-button>
          <el-button type="warning" link :icon="KeyIcon" @click="openResetPassword(row)">重置密码</el-button>
          <el-button type="success" link :icon="UserFilled" @click="openRoleDialog(row)">角色</el-button>
          <el-button type="primary" link :icon="OfficeBuilding" @click="openDeptDialog(row)">部门</el-button>
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
    <el-dialog v-model="dialogVisible" :title="isEdit ? '编辑用户' : '新增用户'" width="500px">
      <el-form :model="formData" label-width="100px">
        <el-form-item label="用户名" required>
          <el-input v-model="formData.username" :disabled="isEdit" placeholder="登录用户名" />
        </el-form-item>
        <el-form-item v-if="!isEdit" label="密码" required>
          <el-input v-model="formData.password" type="password" show-password placeholder="默认 123456" />
        </el-form-item>
        <el-form-item label="显示名称" required>
          <el-input v-model="formData.display_name" placeholder="用于展示的名称" />
        </el-form-item>
        <el-form-item label="邮箱">
          <el-input v-model="formData.email" placeholder="电子邮箱" />
        </el-form-item>
        <el-form-item label="手机号">
          <el-input v-model="formData.phone" placeholder="手机号码" />
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

    <!-- 重置密码弹窗 -->
    <el-dialog v-model="pwdVisible" :title="`重置密码 - ${pwdUserName}`" width="400px">
      <el-form label-width="100px">
        <el-form-item label="新密码" required>
          <el-input v-model="newPassword" type="password" show-password />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="pwdVisible = false">取消</el-button>
        <el-button type="primary" @click="handleResetPassword">确定</el-button>
      </template>
    </el-dialog>

    <!-- 角色管理弹窗 -->
    <el-dialog v-model="roleVisible" :title="`角色分配 - ${roleUserName}`" width="500px">
      <el-checkbox-group v-if="allRoles.length > 0">
        <div v-for="role in allRoles" :key="role.id" style="margin-bottom: 12px;">
          <el-checkbox
            :model-value="isRoleAssigned(role.id)"
            @change="toggleRole(role)"
          >
            <strong>{{ role.display_name || role.name }}</strong>
            <span v-if="role.description" style="color: #909399; margin-left: 8px;">
              ({{ role.description }})
            </span>
          </el-checkbox>
        </div>
      </el-checkbox-group>
      <el-empty v-else description="暂无可用角色" />
    </el-dialog>

    <!-- 部门分配弹窗 -->
    <el-dialog v-model="deptVisible" :title="`分配部门 - ${deptUserName}`" width="500px">
      <div v-loading="deptLoading">
        <el-tree
          v-if="deptTreeData.length > 0"
          ref="deptTreeRef"
          :data="deptTreeData"
          :props="{ label: 'name', children: 'children' }"
          show-checkbox
          node-key="id"
          :default-checked-keys="deptCheckedKeys"
          default-expand-all
        />
        <el-empty v-else-if="!deptLoading" description="暂无可用部门" />
      </div>
      <template #footer>
        <el-button @click="deptVisible = false">取消</el-button>
        <el-button type="primary" :loading="deptLoading" @click="handleSaveDepts">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
.user-manager {
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
