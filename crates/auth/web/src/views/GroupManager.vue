<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { Plus, Refresh, Delete } from '@element-plus/icons-vue'
import { getGroups, addGroup, deleteGroup, type PolicyRecord, type PolicyDto } from '../api/auth'

const loading = ref(false)
const groups = ref<PolicyRecord[]>([])

const dialogVisible = ref(false)
const formData = ref({
  v0: '',
  v1: ''
})

async function loadGroups() {
  loading.value = true
  try {
    groups.value = await getGroups()
  } catch (error: any) {
    ElMessage.error(error.message || '加载资源分组失败')
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
    ElMessage.warning('请填写资源路径和分组名')
    return
  }
  loading.value = true
  try {
    const dto: PolicyDto = {
      ptype: 'g2',
      v0: formData.value.v0,
      v1: formData.value.v1,
      v2: ''
    }
    await addGroup(dto)
    ElMessage.success('添加资源分组成功')
    dialogVisible.value = false
    await loadGroups()
  } catch (error: any) {
    ElMessage.error(error.message || '添加资源分组失败')
  } finally {
    loading.value = false
  }
}

async function handleDelete(id: number) {
  loading.value = true
  try {
    await deleteGroup(id)
    ElMessage.success('删除资源分组成功')
    await loadGroups()
  } catch (error: any) {
    ElMessage.error(error.message || '删除资源分组失败')
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  loadGroups()
})
</script>

<template>
  <div class="group-manager">
    <div class="page-header">
      <h2 class="page-title">资源分组管理</h2>
      <p class="page-desc">管理资源路径与分组的映射关系（Casbin g2 类型策略）</p>
    </div>
    
    <div class="toolbar">
      <el-button type="primary" :icon="Plus" @click="openAddDialog">添加分组</el-button>
      <el-button :icon="Refresh" @click="loadGroups">刷新</el-button>
    </div>

    <el-table
      v-loading="loading"
      :data="groups"
      border
      stripe
      class="group-table"
    >
      <el-table-column prop="id" label="ID" width="80" />
      <el-table-column prop="ptype" label="类型" width="80" />
      <el-table-column prop="v0" label="资源路径" min-width="200">
        <template #default="{ row }">
          <el-tag type="info">{{ row.v0 }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="v1" label="分组名" min-width="150">
        <template #default="{ row }">
          <el-tag type="warning">{{ row.v1 }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="100" fixed="right">
        <template #default="{ row }">
          <el-popconfirm
            title="确定要删除这条分组映射吗？"
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
      title="添加资源分组"
      width="450px"
    >
      <el-form :model="formData" label-width="80px">
        <el-form-item label="资源路径" required>
          <el-input v-model="formData.v0" placeholder="输入资源路径，如 /api/users" />
        </el-form-item>
        <el-form-item label="分组名" required>
          <el-input v-model="formData.v1" placeholder="输入分组名称，如 user_module" />
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
.group-manager {
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

.group-table {
  flex: 1;
}
</style>
