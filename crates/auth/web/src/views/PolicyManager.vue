<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { Plus, Refresh, Delete } from '@element-plus/icons-vue'
import { getPolicies, addPolicy, deletePolicy, type PolicyRecord, type PolicyDto } from '../api/auth'

const loading = ref(false)
const policies = ref<PolicyRecord[]>([])
const searchKeyword = ref('')

const dialogVisible = ref(false)
const formData = ref<PolicyDto>({
  ptype: 'p',
  v0: '',
  v1: '',
  v2: 'get',
  v3: 'allow',
  v4: '',
  v5: ''
})

const actionOptions = ['get', 'post', 'put', 'delete', 'patch', '*']
const effectOptions = ['allow', 'deny']

const filteredPolicies = computed(() => {
  if (!searchKeyword.value.trim()) {
    return policies.value
  }
  const keyword = searchKeyword.value.toLowerCase()
  return policies.value.filter(p =>
    p.v0?.toLowerCase().includes(keyword) ||
    p.v1?.toLowerCase().includes(keyword) ||
    p.v2?.toLowerCase().includes(keyword)
  )
})

async function loadPolicies() {
  loading.value = true
  try {
    policies.value = await getPolicies()
  } catch (error: any) {
    ElMessage.error(error.message || '加载策略列表失败')
  } finally {
    loading.value = false
  }
}

function openAddDialog() {
  formData.value = {
    ptype: 'p',
    v0: '',
    v1: '',
    v2: 'get',
    v3: 'allow',
    v4: '',
    v5: ''
  }
  dialogVisible.value = true
}

async function handleAdd() {
  if (!formData.value.v0 || !formData.value.v1 || !formData.value.v2) {
    ElMessage.warning('请填写必要字段')
    return
  }
  loading.value = true
  try {
    await addPolicy(formData.value)
    ElMessage.success('添加策略成功')
    dialogVisible.value = false
    await loadPolicies()
  } catch (error: any) {
    ElMessage.error(error.message || '添加策略失败')
  } finally {
    loading.value = false
  }
}

async function handleDelete(id: number) {
  loading.value = true
  try {
    await deletePolicy(id)
    ElMessage.success('删除策略成功')
    await loadPolicies()
  } catch (error: any) {
    ElMessage.error(error.message || '删除策略失败')
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  loadPolicies()
})
</script>

<template>
  <div class="policy-manager">
    <div class="page-header">
      <h2 class="page-title">策略管理</h2>
    </div>
    
    <div class="toolbar">
      <el-input
        v-model="searchKeyword"
        placeholder="按 Subject/Object/Action 搜索"
        clearable
        class="search-input"
      />
      <el-button type="primary" :icon="Plus" @click="openAddDialog">新增策略</el-button>
      <el-button :icon="Refresh" @click="loadPolicies">刷新</el-button>
    </div>

    <el-table
      v-loading="loading"
      :data="filteredPolicies"
      border
      stripe
      class="policy-table"
    >
      <el-table-column prop="id" label="ID" width="80" />
      <el-table-column prop="ptype" label="类型" width="80" />
      <el-table-column prop="v0" label="Subject" min-width="120" />
      <el-table-column prop="v1" label="Object" min-width="180" />
      <el-table-column prop="v2" label="Action" width="100" />
      <el-table-column prop="v3" label="Effect" width="100" />
      <el-table-column prop="v4" label="v4" width="100" />
      <el-table-column prop="v5" label="v5" width="100" />
      <el-table-column label="操作" width="100" fixed="right">
        <template #default="{ row }">
          <el-popconfirm
            title="确定要删除这条策略吗？"
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
      title="新增策略"
      width="500px"
    >
      <el-form :model="formData" label-width="100px">
        <el-form-item label="类型 (ptype)">
          <el-select v-model="formData.ptype" style="width: 100%">
            <el-option label="p (策略)" value="p" />
            <el-option label="p2 (策略2)" value="p2" />
          </el-select>
        </el-form-item>
        <el-form-item label="Subject (v0)" required>
          <el-input v-model="formData.v0" placeholder="例如：admin, user, role:admin" />
        </el-form-item>
        <el-form-item label="Object (v1)" required>
          <el-input v-model="formData.v1" placeholder="例如：/api/users, /api/orders/*" />
        </el-form-item>
        <el-form-item label="Action (v2)" required>
          <el-select v-model="formData.v2" style="width: 100%">
            <el-option v-for="opt in actionOptions" :key="opt" :label="opt" :value="opt" />
          </el-select>
        </el-form-item>
        <el-form-item label="Effect (v3)">
          <el-select v-model="formData.v3" style="width: 100%" allow-create filterable>
            <el-option v-for="opt in effectOptions" :key="opt" :label="opt" :value="opt" />
          </el-select>
        </el-form-item>
        <el-form-item label="v4">
          <el-input v-model="formData.v4" placeholder="可选字段" />
        </el-form-item>
        <el-form-item label="v5">
          <el-input v-model="formData.v5" placeholder="可选字段" />
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
.policy-manager {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.page-header {
  margin-bottom: 20px;
}

.page-title {
  margin: 0;
  font-size: 18px;
  color: #303133;
}

.toolbar {
  display: flex;
  gap: 10px;
  margin-bottom: 20px;
}

.search-input {
  width: 300px;
}

.policy-table {
  flex: 1;
}
</style>
