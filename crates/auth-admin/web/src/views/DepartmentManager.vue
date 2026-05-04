<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Refresh, Plus, Delete, Edit, Top, User } from '@element-plus/icons-vue'
import {
  getDepartments, createDepartment, updateDepartment, deleteDepartment, moveDepartment,
  getDepartmentUsers,
  type DepartmentRecord
} from '../api'

const loading = ref(false)
const deptList = ref<DepartmentRecord[]>([])

async function loadDepts() {
  loading.value = true
  try {
    deptList.value = await getDepartments()
  } catch (e: any) {
    ElMessage.error(e.message || '加载部门列表失败')
  } finally {
    loading.value = false
  }
}

// 将平铺列表转为树形（用于 el-table 树形展示）
function buildTree(list: DepartmentRecord[], parentId: number = 0): (DepartmentRecord & { children?: DepartmentRecord[] })[] {
  return list
    .filter(d => (d.parent_id ?? 0) === parentId)
    .map(d => {
      const children = buildTree(list, d.id)
      return children.length > 0 ? { ...d, children } : { ...d }
    })
}
const treeData = computed(() => buildTree(deptList.value))

// 创建/编辑弹窗
const dialogVisible = ref(false)
const isEdit = ref(false)
const editId = ref<number | null>(null)
const formData = ref({
  name: '', parent_id: undefined as number | undefined, sort_order: 0, description: '', status: 1
})

function openCreate(parentId?: number) {
  isEdit.value = false
  editId.value = null
  formData.value = { name: '', parent_id: parentId, sort_order: 0, description: '', status: 1 }
  dialogVisible.value = true
}

function openEdit(row: DepartmentRecord) {
  isEdit.value = true
  editId.value = row.id
  formData.value = {
    name: row.name,
    parent_id: row.parent_id,
    sort_order: row.sort_order || 0,
    description: row.description || '',
    status: row.status
  }
  dialogVisible.value = true
}

async function handleSave() {
  if (!formData.value.name) {
    ElMessage.warning('请填写部门名称')
    return
  }
  loading.value = true
  try {
    if (isEdit.value && editId.value) {
      await updateDepartment(editId.value, formData.value)
      ElMessage.success('更新成功')
    } else {
      await createDepartment(formData.value)
      ElMessage.success('创建成功')
    }
    dialogVisible.value = false
    loadDepts()
  } catch (e: any) {
    ElMessage.error(e.message || '保存失败')
  } finally {
    loading.value = false
  }
}

async function handleDelete(row: DepartmentRecord) {
  try {
    await ElMessageBox.confirm(`确定要删除部门 "${row.name}" 吗？`, '确认删除', {
      type: 'warning', confirmButtonText: '确定', cancelButtonText: '取消'
    })
    await deleteDepartment(row.id)
    ElMessage.success('删除成功')
    loadDepts()
  } catch { /* 取消 */ }
}

// 移动部门弹窗
const moveVisible = ref(false)
const moveDeptId = ref(0)
const moveDeptName = ref('')
const moveTargetId = ref<number | undefined>(undefined)

function openMove(row: DepartmentRecord) {
  moveDeptId.value = row.id
  moveDeptName.value = row.name
  moveTargetId.value = row.parent_id
  moveVisible.value = true
}

async function handleMove() {
  if (moveTargetId.value === moveDeptId.value) {
    ElMessage.warning('不能移动到自身')
    return
  }
  try {
    await moveDepartment(moveDeptId.value, moveTargetId.value ?? 0)
    ElMessage.success('移动成功')
    moveVisible.value = false
    loadDepts()
  } catch (e: any) {
    ElMessage.error(e.message || '移动失败')
  }
}

onMounted(() => { loadDepts() })

// 查看部门成员
const memberDialogVisible = ref(false)
const memberLoading = ref(false)
const currentDeptName = ref('')
const departmentMembers = ref<any[]>([])

async function handleViewMembers(row: DepartmentRecord) {
  currentDeptName.value = row.name
  memberDialogVisible.value = true
  memberLoading.value = true
  try {
    departmentMembers.value = await getDepartmentUsers(row.id)
  } catch (e: any) {
    ElMessage.error('获取部门成员失败: ' + (e.message || e))
  } finally {
    memberLoading.value = false
  }
}
</script>

<template>
  <div class="dept-manager">
    <div class="page-header">
      <h2 class="page-title">组织架构管理</h2>
    </div>

    <!-- 工具栏 -->
    <div class="toolbar">
      <el-button type="primary" :icon="Plus" @click="openCreate()">新增部门</el-button>
      <el-button :icon="Refresh" @click="loadDepts">刷新</el-button>
    </div>

    <!-- 表格 -->
    <el-table v-loading="loading" :data="treeData" border stripe class="data-table"
      row-key="id" :tree-props="{ children: 'children', hasChildren: 'hasChildren' }" default-expand-all>
      <el-table-column prop="name" label="部门名称" min-width="200" />
      <el-table-column prop="sort_order" label="排序" width="80" />
      <el-table-column prop="member_count" label="成员数量" min-width="100">
        <template #default="{ row }">
          <el-link v-if="row.member_count" type="primary" :underline="false" @click="handleViewMembers(row)">
            {{ row.member_count }}
          </el-link>
          <span v-else>0</span>
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
      <el-table-column label="操作" width="340" fixed="right">
        <template #default="{ row }">
          <el-button type="primary" link :icon="User" @click="handleViewMembers(row)">成员</el-button>
          <el-button type="success" link :icon="Plus" @click="openCreate(row.id)">添加子部门</el-button>
          <el-button type="primary" link :icon="Edit" @click="openEdit(row)">编辑</el-button>
          <el-button type="warning" link :icon="Top" @click="openMove(row)">移动</el-button>
          <el-button type="danger" link :icon="Delete" @click="handleDelete(row)">删除</el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- 创建/编辑弹窗 -->
    <el-dialog v-model="dialogVisible" :title="isEdit ? '编辑部门' : '新增部门'" width="500px">
      <el-form :model="formData" label-width="100px">
        <el-form-item label="部门名称" required>
          <el-input v-model="formData.name" placeholder="部门名称" />
        </el-form-item>
        <el-form-item label="父部门">
          <el-select v-model="formData.parent_id" placeholder="可选（顶级部门）" clearable style="width: 100%">
            <el-option v-for="d in deptList.filter(d => d.id !== editId)" :key="d.id"
              :label="d.name" :value="d.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="排序">
          <el-input-number v-model="formData.sort_order" :min="0" />
        </el-form-item>
        <el-form-item label="描述">
          <el-input v-model="formData.description" type="textarea" placeholder="部门描述" />
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

    <!-- 移动部门弹窗 -->
    <el-dialog v-model="moveVisible" :title="`移动部门 - ${moveDeptName}`" width="420px">
      <el-form label-width="100px">
        <el-form-item label="目标父部门">
          <el-select v-model="moveTargetId" placeholder="选择父部门（清空为顶级）" clearable style="width: 100%">
            <el-option v-for="d in deptList.filter(d => d.id !== moveDeptId)" :key="d.id"
              :label="d.name" :value="d.id" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="moveVisible = false">取消</el-button>
        <el-button type="primary" @click="handleMove">确定</el-button>
      </template>
    </el-dialog>

    <!-- 部门成员弹窗 -->
    <el-dialog v-model="memberDialogVisible" :title="`部门成员 - ${currentDeptName}`" width="700px">
      <el-table v-loading="memberLoading" :data="departmentMembers" border stripe>
        <el-table-column prop="username" label="用户名" min-width="120" />
        <el-table-column prop="display_name" label="显示名称" min-width="120" />
        <el-table-column prop="email" label="邮箱" min-width="180" show-overflow-tooltip />
        <el-table-column prop="phone" label="手机号" min-width="130" />
        <el-table-column label="状态" width="90">
          <template #default="{ row }">
            <el-tag :type="row.status === 1 ? 'success' : 'danger'" size="small">
              {{ row.status === 1 ? '启用' : '禁用' }}
            </el-tag>
          </template>
        </el-table-column>
        <template #empty>
          <el-empty description="该部门暂无成员" :image-size="80" />
        </template>
      </el-table>
      <template #footer>
        <el-button @click="memberDialogVisible = false">关闭</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
.dept-manager {
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
