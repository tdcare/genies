import { createRouter, createWebHashHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/login',
    name: 'Login',
    component: () => import('../views/Login.vue')
  },
  {
    path: '/',
    redirect: '/apps'
  },
  {
    path: '/apps',
    name: 'ApplicationList',
    component: () => import('../views/ApplicationList.vue'),
    meta: { title: '应用管理', requireAuth: true }
  },
  {
    path: '/apps/:id/permissions',
    name: 'ApplicationPermission',
    component: () => import('../views/ApplicationPermission.vue'),
    meta: { title: '权限管理', requireAuth: true }
  },
  {
    path: '/users',
    name: 'UserManager',
    component: () => import('../views/UserManager.vue')
  },
  {
    path: '/roles',
    name: 'RoleManager',
    component: () => import('../views/RoleManager.vue')
  },
  {
    path: '/permissions',
    name: 'PermissionManager',
    component: () => import('../views/PermissionManager.vue')
  },
  {
    path: '/departments',
    name: 'DepartmentManager',
    component: () => import('../views/DepartmentManager.vue')
  },
  {
    path: '/profile',
    name: 'Profile',
    component: () => import('../views/Profile.vue')
  }
]

const router = createRouter({
  history: createWebHashHistory(),
  routes
})

// 路由守卫：未登录跳转登录页
router.beforeEach((to, _from, next) => {
  const token = localStorage.getItem('admin_token')
  if (to.path !== '/login' && !token) {
    next('/login')
  } else if (to.path === '/login' && token) {
    next('/apps')
  } else {
    next()
  }
})

export default router
