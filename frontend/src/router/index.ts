import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/login', name: 'Login', component: () => import('@/pages/LoginPage.vue'), meta: { public: true } },
    { path: '/change-password', name: 'ChangePassword', component: () => import('@/pages/ChangePasswordPage.vue') },
    {
      path: '/',
      component: () => import('@/components/layout/AppLayout.vue'),
      children: [
        { path: '', redirect: '/dashboard' },
        { path: 'dashboard', name: 'Dashboard', component: () => import('@/pages/DashboardPage.vue') },
        { path: 'monitors/new', name: 'MonitorNew', component: () => import('@/pages/MonitorFormPage.vue') },
        { path: 'monitors/:id', name: 'MonitorDetail', component: () => import('@/pages/MonitorDetailPage.vue') },
        { path: 'monitors/:id/edit', name: 'MonitorEdit', component: () => import('@/pages/MonitorFormPage.vue') },
        { path: 'incidents', name: 'Incidents', component: () => import('@/pages/IncidentsPage.vue') },
        { path: 'maintenance', name: 'Maintenance', component: () => import('@/pages/MaintenancePage.vue') },
        { path: 'notifications', name: 'Notifications', component: () => import('@/pages/NotificationsPage.vue') },
        { path: 'users', name: 'Users', component: () => import('@/pages/UsersPage.vue') },
        { path: 'api-keys', name: 'ApiKeys', component: () => import('@/pages/ApiKeysPage.vue') },
        { path: 'audit-log', name: 'AuditLog', component: () => import('@/pages/AuditLogPage.vue') },
        { path: 'settings', name: 'Settings', component: () => import('@/pages/SettingsPage.vue') },
      ],
    },
    { path: '/:pathMatch(.*)*', redirect: '/dashboard' },
  ],
})

router.beforeEach(async (to) => {
  const auth = useAuthStore()

  if (to.meta.public) return true

  const token = localStorage.getItem('access_token')
  if (!token) return { name: 'Login' }

  if (!auth.user) {
    try {
      await auth.fetchMe()
    } catch {
      return { name: 'Login' }
    }
  }

  // Force password change
  if (auth.user?.force_pw_change && to.name !== 'ChangePassword') {
    return { name: 'ChangePassword' }
  }

  return true
})

export { router }
