<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { Plus, Refresh, Delete } from '@element-plus/icons-vue'
import { getRoles, addRole, deleteRole, type PolicyRecord, type PolicyDto } from '../api/auth'

const loading = ref(false)
const roles = ref<PolicyRecord[]>([])

const dialogVisible = ref(false)
const formData = ref({
  v0: '',
  v1: ''
})

async function loadRoles() {
  loading.value = true
  try {
    roles.value = await getRoles()
  } catch (error: any) {
    ElMessage.error(error.message || '加载角色列表失败')
  } finally {
    loading.value = false
  }
}

function openAddDialog() {
  formData.value = { v0: '', v1: '' }
  dialogVisible.value = true
}

async function handleAdd() {
  if (!formData.value.v0 || !formData.value.v1) {
    ElMessage.warning('请填写用户名和角色名')
    return
  }
  loading.value = true
  try {
    const dto: PolicyDto = {
      ptype: 'g',
      v0: formData.value.v0,
      v1: formData.value.v1,
      v2: ''
    }
    await addRole(dto)
    ElMessage.success('添加角色分配成功')
    dialogVisible.value = false
    await loadRoles()
  } catch (error: any) {
    ElMessage.error(error.message || '添加角色分配失败')
  } finally {
    loading.value = false
  }
}

async function handleDelete(id: number) {
  loading.value = true
  try {
    await deleteRole(id)
    ElMessage.success('删除角色分配成功')
    await loadRoles()
  } catch (error: any) {
    ElMessage.error(error.message || '删除角色分配失败')
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  loadRoles()
})
</script>

<template>
  <div class="role-manager">
    <div class="page-header">
      <h2 class="page-title">角色管理</h2>
      <p class="page-desc">管理用户与角色的关联关系（Casbin g 类型策略）</p>
    </div>
    
    <div class="toolbar">
      <el-button type="primary" :icon="Plus" @click="openAddDialog">添加角色分配</el-button>
      <el-button :icon="Refresh" @click="loadRoles">刷新</el-button>
    </div>

    <el-table
      v-loading="loading"
      :data="roles"
      border
      stripe
      class="role-table"
    >
      <el-table-column prop="id" label="ID" width="80" />
      <el-table-column prop="ptype" label="类型" width="80" />
      <el-table-column prop="v0" label="用户" min-width="150">
        <template #default="{ row }">
          <el-tag>{{ row.v0 }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="v1" label="角色" min-width="150">
        <template #default="{ row }">
          <el-tag type="success">{{ row.v1 }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="100" fixed="right">
        <template #default="{ row }">
          <el-popconfirm
            title="确定要删除这条角色分配吗？"
            confirm-button-text="确定"
            cancel-button-text="取消"
            @confirm="handleDelete(row.id)"
          >
            <template #reference>
              <el-button type="danger" :icon="Delete" link>删除</el-button>
            </template>
          </el-popconfirm>
        </template>
      </el-table-column>
    </el-table>

    <el-dialog
      v-model="dialogVisible"
      title="添加角色分配"
      width="450px"
    >
      <el-form :model="formData" label-width="80px">
        <el-form-item label="用户名" required>
          <el-input v-model="formData.v0" placeholder="输入用户名或用户标识" />
        </el-form-item>
        <el-form-item label="角色名" required>
          <el-input v-model="formData.v1" placeholder="输入角色名称，如 admin, editor" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="loading" @click="handleAdd">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
.role-manager {
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

.toolbar {
  display: flex;
  gap: 10px;
  margin-bottom: 20px;
}

.role-table {
  flex: 1;
}
</style>
