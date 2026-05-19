<script setup lang="ts">
import { ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { User, UserFilled, Key, Lock, OfficeBuilding, SwitchButton, Grid, Setting } from '@element-plus/icons-vue'
import { changePassword } from '../api'
import { getApiBaseUrl } from '../utils/path'
import axios from 'axios'

// 独立的 axios 实例用于 logout，不经过拦截器避免 token 刷新死锁
const logoutApi = axios.create({ baseURL: getApiBaseUrl(), timeout: 5000 })

const route = useRoute()
const router = useRouter()

const userInfo = JSON.parse(localStorage.getItem('admin_user') || '{}')

const menuItems = [
  { index: '/apps', title: '应用管理', icon: Grid },
  { index: '/users', title: '用户管理', icon: User },
  { index: '/roles', title: '角色管理', icon: UserFilled },
  { index: '/permissions', title: '权限管理', icon: Key },
  { index: '/departments', title: '组织架构', icon: OfficeBuilding },
  { index: '/settings', title: '系统设置', icon: Setting }
]

function handleMenuSelect(index: string) {
  router.push(index)
}

async function handleLogout() {
  // 先清除 token，确保即使 API 失败也能退出
  localStorage.removeItem('admin_token')
  localStorage.removeItem('admin_token_expires_at')
  localStorage.removeItem('admin_user')

  try {
    await logoutApi.post('/logout')
  } catch {
    // ignore
  }

  ElMessage.success('已退出登录')
  router.push('/login')
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
</script>

<template>
  <el-container class="layout-container">
    <el-header class="layout-header">
      <div class="header-left">
        <h1 class="header-title">统一认证管理</h1>
      </div>
      <div class="header-right">
        <span class="header-user">{{ userInfo.display_name || userInfo.username || '管理员' }}</span>
        <el-button type="primary" link @click="router.push('/profile')">个人信息</el-button>
        <el-button type="warning" link @click="pwdVisible = true">修改密码</el-button>
        <el-button type="danger" link :icon="SwitchButton" @click="handleLogout">退出</el-button>
      </div>
    </el-header>
    <el-container class="layout-body">
      <el-aside width="200px" class="layout-aside">
        <el-menu
          :default-active="route.path"
          router
          class="aside-menu"
          @select="handleMenuSelect"
        >
          <el-menu-item v-for="item in menuItems" :key="item.index" :index="item.index">
            <el-icon><component :is="item.icon" /></el-icon>
            <span>{{ item.title }}</span>
          </el-menu-item>
        </el-menu>
      </el-aside>
      <el-main class="layout-main">
        <slot />
      </el-main>
    </el-container>
  </el-container>

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
</template>

<style scoped>
.layout-container { height: 100%; }

.layout-header {
  background-color: #409eff;
  color: white;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 20px;
}

.header-left { display: flex; align-items: center; }

.header-title {
  margin: 0;
  font-size: 20px;
  font-weight: 500;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 10px;
}

.header-user {
  font-size: 14px;
  opacity: 0.9;
}

.layout-body {
  height: calc(100% - 60px);
}

.layout-aside {
  background-color: #545c64;
  overflow-y: auto;
}

.aside-menu {
  height: 100%;
  border-right: none;
  background-color: #545c64;
}

.aside-menu .el-menu-item {
  color: #fff;
}

.aside-menu .el-menu-item:hover {
  background-color: #434a50;
}

.aside-menu .el-menu-item.is-active {
  background-color: #409eff;
  color: #fff;
}

.layout-main {
  background-color: #f5f7fa;
  padding: 20px;
  overflow-y: auto;
}
</style>
