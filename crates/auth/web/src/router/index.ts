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

const router = createRouter({
  history: createWebHistory('/auth/ui/'),
  routes
})

export default router
