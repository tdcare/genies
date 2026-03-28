<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { Refresh } from '@element-plus/icons-vue'
import { getModel, updateModel, reloadEnforcer, type ModelRecord, type ModelDto } from '../api/auth'

const loading = ref(false)
const reloading = ref(false)
const model = ref<ModelRecord | null>(null)

const formData = ref<ModelDto>({
  model_name: '',
  model_text: '',
  description: ''
})

async function loadModel() {
  loading.value = true
  try {
    const m = await getModel()
    model.value = m
    formData.value = {
      model_name: m.model_name,
      model_text: m.model_text,
      description: m.description ?? ''
    }
  } catch (error: any) {
    ElMessage.error(error.message || '加载模型配置失败')
  } finally {
    loading.value = false
  }
}

async function handleSave() {
  if (!formData.value.model_name || !formData.value.model_text) {
    ElMessage.warning('请填写模型名称和模型内容')
    return
  }
  loading.value = true
  try {
    await updateModel(formData.value)
    ElMessage.success('保存模型成功')
    await loadModel()
  } catch (error: any) {
    ElMessage.error(error.message || '保存模型失败')
  } finally {
    loading.value = false
  }
}

async function handleReload() {
  reloading.value = true
  try {
    await reloadEnforcer()
    ElMessage.success('Enforcer 重载成功')
  } catch (error: any) {
    ElMessage.error(error.message || '重载 Enforcer 失败')
  } finally {
    reloading.value = false
  }
}

onMounted(() => {
  loadModel()
})
</script>

<template>
  <div class="model-config" v-loading="loading">
    <div class="page-header">
      <h2 class="page-title">模型配置</h2>
      <p class="page-desc">配置和管理 Casbin 权限模型</p>
    </div>

    <div class="toolbar">
      <el-button :icon="Refresh" @click="loadModel">刷新</el-button>
    </div>

    <el-card class="model-card">
      <template #header>
        <div class="card-header">
          <span>Casbin 模型配置</span>
        </div>
      </template>

      <el-form :model="formData" label-width="100px" class="model-form">
        <el-form-item label="模型名称" required>
          <el-input v-model="formData.model_name" placeholder="例如：rbac_model" />
        </el-form-item>

        <el-form-item label="描述">
          <el-input v-model="formData.description" placeholder="模型描述（可选）" />
        </el-form-item>

        <el-form-item label="模型内容" required>
          <el-input
            v-model="formData.model_text"
            type="textarea"
            :rows="18"
            placeholder="请输入 Casbin 模型配置（INI 格式）"
            class="model-textarea"
          />
        </el-form-item>

        <el-form-item>
          <el-button type="primary" :loading="loading" @click="handleSave">
            保存模型
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-divider />

    <el-card class="reload-card">
      <template #header>
        <div class="card-header">
          <span>Enforcer 管理</span>
        </div>
      </template>

      <div class="reload-section">
        <p class="reload-desc">
          当你修改了模型配置或策略规则后，需要重载 Enforcer 使更改生效。
          重载操作会重新加载模型和所有策略。
        </p>
        <el-button
          type="warning"
          :loading="reloading"
          @click="handleReload"
          size="large"
        >
          重载 Enforcer
        </el-button>
      </div>
    </el-card>

    <el-card class="help-card">
      <template #header>
        <div class="card-header">
          <span>模型配置示例</span>
        </div>
      </template>

      <div class="help-content">
        <p>RBAC 模型示例：</p>
        <pre class="code-block">[request_definition]
r = sub, obj, act

[policy_definition]
p = sub, obj, act

[role_definition]
g = _, _

[policy_effect]
e = some(where (p.eft == allow))

[matchers]
m = g(r.sub, p.sub) && r.obj == p.obj && r.act == p.act</pre>
      </div>
    </el-card>
  </div>
</template>

<style scoped>
.model-config {
  height: 100%;
  overflow-y: auto;
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

.model-card {
  margin-bottom: 20px;
}

.card-header {
  font-weight: 600;
}

.model-form {
  max-width: 800px;
}

.model-textarea :deep(textarea) {
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  font-size: 13px;
  line-height: 1.5;
}

.reload-card {
  margin-bottom: 20px;
}

.reload-section {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 15px;
}

.reload-desc {
  margin: 0;
  color: #606266;
  font-size: 14px;
}

.help-card {
  margin-bottom: 20px;
}

.help-content {
  font-size: 14px;
  color: #606266;
}

.help-content p {
  margin: 0 0 10px 0;
}

.code-block {
  background-color: #f5f7fa;
  padding: 15px;
  border-radius: 4px;
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  font-size: 13px;
  line-height: 1.6;
  overflow-x: auto;
  margin: 0;
  white-space: pre;
}
</style>
