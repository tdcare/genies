<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Delete } from '@element-plus/icons-vue'
import { getPolicies, addPolicy, deletePolicy, getRoles, type PolicyRecord } from '../api/auth'

const props = defineProps<{
  visible: boolean
  targetType: 'api' | 'field'
  targetName: string
  objectValue: string
  defaultAction: string
}>()

const emit = defineEmits<{
  (e: 'update:visible', value: boolean): void
  (e: 'changed'): void
}>()

const dialogVisible = computed({
  get: () => props.visible,
  set: (value) => emit('update:visible', value)
})

// 表单数据
const formData = ref({
  subject: '',
  action: '',
  effect: 'deny'
})

// 角色建议列表
const roleSuggestions = ref<string[]>([])
const loadingRoles = ref(false)

// 权限列表
const policies = ref<PolicyRecord[]>([])
const loadingPolicies = ref(false)

// Action 选项
const actionOptions = computed(() => {
  if (props.targetType === 'api') {
    return ['get', 'post', 'put', 'delete', 'patch', '*']
  } else {
    return ['read', 'write', '*']
  }
})

// 过滤后的权限列表（匹配 objectValue）
const filteredPolicies = computed(() => {
  return policies.value.filter(p => p.v1 === props.objectValue && p.ptype === 'p')
})

// 加载角色列表（用于 Subject 建议）
async function loadRoles() {
  loadingRoles.value = true
  try {
    const roles = await getRoles()
    // 提取所有唯一的 subject（v0 和 v1 都可能是 subject/role）
    const subjects = new Set<string>()
    roles.forEach(r => {
      if (r.v0) subjects.add(r.v0)
      if (r.v1) subjects.add(r.v1)
    })
    roleSuggestions.value = Array.from(subjects)
  } catch (error: any) {
    console.error('加载角色失败', error)
  } finally {
    loadingRoles.value = false
  }
}

// 加载策略列表
async function loadPolicies() {
  loadingPolicies.value = true
  try {
    policies.value = await getPolicies()
  } catch (error: any) {
    ElMessage.error(error.message || '加载策略列表失败')
  } finally {
    loadingPolicies.value = false
  }
}

// 添加权限
async function handleAddPolicy() {
  if (!formData.value.subject.trim()) {
    ElMessage.warning('请输入 Subject')
    return
  }
  
  try {
    await addPolicy({
      ptype: 'p',
      v0: formData.value.subject.trim(),
      v1: props.objectValue,
      v2: formData.value.action,
      v3: formData.value.effect
    })
    ElMessage.success('添加成功')
    formData.value.subject = ''
    await loadPolicies()
    emit('changed')
  } catch (error: any) {
    ElMessage.error(error.message || '添加失败')
  }
}

// 删除权限
async function handleDeletePolicy(policy: PolicyRecord) {
  try {
    await ElMessageBox.confirm(
      `确认删除权限: ${policy.v0} -> ${policy.v2} (${policy.v3})?`,
      '确认删除',
      { type: 'warning' }
    )
    await deletePolicy(policy.id)
    ElMessage.success('删除成功')
    await loadPolicies()
    emit('changed')
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '删除失败')
    }
  }
}

// Subject 自动完成过滤
function querySubjects(queryString: string, cb: (results: { value: string }[]) => void) {
  const results = queryString
    ? roleSuggestions.value
        .filter(r => r.toLowerCase().includes(queryString.toLowerCase()))
        .map(r => ({ value: r }))
    : roleSuggestions.value.map(r => ({ value: r }))
  cb(results)
}

// 弹框打开时加载数据
watch(() => props.visible, (newVal) => {
  if (newVal) {
    formData.value.action = props.defaultAction
    formData.value.effect = 'deny'
    formData.value.subject = ''
    loadRoles()
    loadPolicies()
  }
})

// Effect 标签类型
function getEffectType(effect: string): 'success' | 'danger' {
  return effect === 'allow' ? 'success' : 'danger'
}
</script>

<template>
  <el-dialog
    v-model="dialogVisible"
    :title="'权限设置 - ' + targetName"
    width="650px"
    :close-on-click-modal="false"
    class="permission-dialog"
  >
    <!-- 目标信息区 -->
    <div class="target-info">
      <el-tag :type="targetType === 'api' ? 'primary' : 'warning'" size="default">
        {{ targetType === 'api' ? 'API' : '字段' }}
      </el-tag>
      <code class="target-value">{{ objectValue }}</code>
    </div>

    <!-- 添加权限表单 -->
    <div class="add-form">
      <el-form :inline="true" @submit.prevent="handleAddPolicy">
        <el-form-item label="Subject">
          <el-autocomplete
            v-model="formData.subject"
            :fetch-suggestions="querySubjects"
            placeholder="输入或选择角色/用户"
            clearable
            style="width: 140px"
          />
        </el-form-item>
        <el-form-item label="Action">
          <el-select v-model="formData.action" style="width: 100px">
            <el-option
              v-for="action in actionOptions"
              :key="action"
              :label="action"
              :value="action"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="Effect">
          <el-select v-model="formData.effect" style="width: 90px">
            <el-option label="deny" value="deny" />
            <el-option label="allow" value="allow" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" :icon="Plus" @click="handleAddPolicy">添加</el-button>
        </el-form-item>
      </el-form>
    </div>

    <!-- 现有权限列表 -->
    <div class="policy-list">
      <h4>现有权限规则</h4>
      <el-table
        :data="filteredPolicies"
        v-loading="loadingPolicies"
        border
        size="small"
        empty-text="暂无权限规则"
      >
        <el-table-column prop="v0" label="Subject" min-width="120" />
        <el-table-column prop="v2" label="Action" width="100" align="center">
          <template #default="{ row }">
            <code>{{ row.v2 }}</code>
          </template>
        </el-table-column>
        <el-table-column prop="v3" label="Effect" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getEffectType(row.v3)" size="small">
              {{ row.v3 }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="80" align="center">
          <template #default="{ row }">
            <el-button
              type="danger"
              :icon="Delete"
              link
              size="small"
              @click="handleDeletePolicy(row)"
            >
              删除
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </div>

    <template #footer>
      <el-button @click="dialogVisible = false">关闭</el-button>
    </template>
  </el-dialog>
</template>

<style scoped>
.permission-dialog :deep(.el-dialog__body) {
  padding-top: 15px;
}

.target-info {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 20px;
  padding: 12px;
  background: #f5f7fa;
  border-radius: 6px;
}

.target-value {
  font-family: monospace;
  font-size: 14px;
  color: #606266;
  background: #e6e8eb;
  padding: 4px 8px;
  border-radius: 4px;
}

.add-form {
  margin-bottom: 20px;
  padding: 15px;
  background: #fafafa;
  border: 1px solid #ebeef5;
  border-radius: 6px;
}

.add-form :deep(.el-form-item) {
  margin-bottom: 0;
  margin-right: 12px;
}

.add-form :deep(.el-form-item__label) {
  font-size: 13px;
}

.policy-list h4 {
  margin: 0 0 12px 0;
  font-size: 14px;
  color: #303133;
}

.policy-list :deep(.el-table) {
  font-size: 13px;
}
</style>
