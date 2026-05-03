<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { getMe, changePassword } from '../api'

interface UserInfo {
  id?: number
  username: string
  display_name: string
  email?: string
  phone?: string
  avatar?: string
  status: number
}

const userInfo = ref<UserInfo | null>(null)
const loading = ref(false)

async function loadProfile() {
  loading.value = true
  try {
    userInfo.value = await getMe()
  } catch (e: any) {
    ElMessage.error(e.message || '获取信息失败')
  } finally {
    loading.value = false
  }
}

// 修改密码弹窗
const pwdVisible = ref(false)
const pwdForm = ref({ oldPassword: '', newPassword: '', confirmPassword: '' })

async function handleChangePassword() {
  if (!pwdForm.value.oldPassword || !pwdForm.value.newPassword) {
    ElMessage.warning('请填写完整信息')
    return
  }
  if (pwdForm.value.newPassword !== pwdForm.value.confirmPassword) {
    ElMessage.warning('两次密码不一致')
    return
  }
  try {
    await changePassword(pwdForm.value.oldPassword, pwdForm.value.newPassword)
    ElMessage.success('密码修改成功')
    pwdVisible.value = false
    pwdForm.value = { oldPassword: '', newPassword: '', confirmPassword: '' }
  } catch (e: any) {
    ElMessage.error(e.message || '修改失败')
  }
}

onMounted(() => { loadProfile() })
</script>

<template>
  <div class="profile-page">
    <div class="page-header">
      <h2 class="page-title">个人信息</h2>
    </div>

    <el-card v-loading="loading" class="profile-card">
      <template v-if="userInfo">
        <el-descriptions :column="2" border>
          <el-descriptions-item label="用户名">{{ userInfo.username }}</el-descriptions-item>
          <el-descriptions-item label="显示名称">{{ userInfo.display_name }}</el-descriptions-item>
          <el-descriptions-item label="邮箱">{{ userInfo.email || '-' }}</el-descriptions-item>
          <el-descriptions-item label="手机号">{{ userInfo.phone || '-' }}</el-descriptions-item>
          <el-descriptions-item label="状态">
            <el-tag :type="userInfo.status === 1 ? 'success' : 'danger'">
              {{ userInfo.status === 1 ? '启用' : '禁用' }}
            </el-tag>
          </el-descriptions-item>
        </el-descriptions>

        <div style="margin-top: 24px">
          <el-button type="primary" @click="pwdVisible = true">修改密码</el-button>
        </div>
      </template>
    </el-card>

    <!-- 修改密码弹窗 -->
    <el-dialog v-model="pwdVisible" title="修改密码" width="420px">
      <el-form :model="pwdForm" label-width="100px">
        <el-form-item label="旧密码" required>
          <el-input v-model="pwdForm.oldPassword" type="password" show-password />
        </el-form-item>
        <el-form-item label="新密码" required>
          <el-input v-model="pwdForm.newPassword" type="password" show-password />
        </el-form-item>
        <el-form-item label="确认密码" required>
          <el-input v-model="pwdForm.confirmPassword" type="password" show-password />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="pwdVisible = false">取消</el-button>
        <el-button type="primary" @click="handleChangePassword">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
.profile-page {
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

.profile-card {
  max-width: 700px;
}
</style>
