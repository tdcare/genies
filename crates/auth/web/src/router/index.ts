import { createRouter, createWebHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/',
    redirect: '/policies'
  },
  {
    path: '/policies',
    name: 'PolicyManager',
    component: () => import('../views/PolicyManager.vue')
  },
  {
    path: '/roles',
    name: 'RoleManager',
    component: () => import('../views/RoleManager.vue')
  },
  {
    path: '/groups',
    name: 'GroupManager',
    component: () => import('../views/GroupManager.vue')
  },
  {
    path: '/schemas',
    name: 'SchemaExplorer',
    component: () => import('../views/SchemaExplorer.vue')
  },
  {
    path: '/model',
    name: 'ModelConfig',
    component: () => import('../views/ModelConfig.vue')
  }
]

/**
 * 从当前页面 URL 动态推断 auth UI 的路由基路径
 * 例如页面在 /sickbed/auth/ui/ 下时，返回 "/sickbed/auth/ui/"
 */
function getRouterBase(): string {
  const path = window.location.pathname
  const idx = path.indexOf('/auth/ui')
  if (idx >= 0) {
    return path.substring(0, idx) + '/auth/ui/'
  }
  return '/auth/ui/'
}

const router = createRouter({
  history: createWebHistory(getRouterBase()),
  routes
})

export default router
