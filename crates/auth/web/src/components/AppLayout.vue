<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { List, User, Folder, Document, Setting } from '@element-plus/icons-vue'
import { getAccessToken } from '../api/auth'

const route = useRoute()
const router = useRouter()

const token = ref('')

async function autoFetchToken() {
  try {
    const result = await getAccessToken()
    token.value = result.access_token
    localStorage.setItem('auth_token', result.access_token)
    // 存储过期时间
    const expiresAt = Date.now() + result.expires_in * 1000
    localStorage.setItem('token_expires_at', String(expiresAt))
    ElMessage.success('已自动获取访问 Token')
  } catch (error: any) {
    ElMessage.warning('自动获取 Token 失败，请手动输入')
  }
}

onMounted(() => {
  const savedToken = localStorage.getItem('auth_token')
  const expiresAt = localStorage.getItem('token_expires_at')
  
  if (savedToken && expiresAt && Date.now() < Number(expiresAt)) {
    // Token 存在且未过期
    token.value = savedToken
  } else {
    // 没有 Token 或已过期，自动获取
    autoFetchToken()
  }
})

function saveToken() {
  if (token.value.trim()) {
    localStorage.setItem('auth_token', token.value.trim())
    // 手动保存的 token 设置较长过期时间（30 天）
    const expiresAt = Date.now() + 30 * 24 * 60 * 60 * 1000
    localStorage.setItem('token_expires_at', String(expiresAt))
    ElMessage.success('Token 已保存')
  } else {
    localStorage.removeItem('auth_token')
    localStorage.removeItem('token_expires_at')
    ElMessage.info('Token 已清除')
  }
}

const menuItems = [
  { index: '/schemas', title: '权限设置', icon: Document },
  { index: '/policies', title: '策略管理', icon: List },
  { index: '/roles', title: '角色管理', icon: User },
  { index: '/groups', title: '资源分组', icon: Folder },
  { index: '/model', title: '模型配置', icon: Setting }
]

function handleMenuSelect(index: string) {
  router.push(index)
}
</script>

<template>
  <el-container class="layout-container">
    <el-header class="layout-header">
      <div class="header-left">
        <h1 class="header-title">Casbin 权限管理</h1>
      </div>
      <div class="header-right">
        <el-input
          v-model="token"
          placeholder="请输入 JWT Token"
          class="token-input"
          type="password"
          show-password
        />
        <el-button type="primary" @click="saveToken">保存 Token</el-button>
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
          <el-menu-item
            v-for="item in menuItems"
            :key="item.index"
            :index="item.index"
          >
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
</template>

<style scoped>
.layout-container {
  height: 100%;
}

.layout-header {
  background-color: #409eff;
  color: white;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 20px;
}

.header-left {
  display: flex;
  align-items: center;
}

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

.token-input {
  width: 300px;
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
