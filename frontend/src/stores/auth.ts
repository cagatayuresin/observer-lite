import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { authApi } from '@/api/auth'

export interface User {
  id: number
  username: string
  email: string
  role: string
  force_pw_change: boolean
}

export const useAuthStore = defineStore('auth', () => {
  const user = ref<User | null>(null)
  const loading = ref(false)

  const isLoggedIn = computed(() => !!user.value)
  const isSuperAdmin = computed(() => user.value?.role === 'superadmin')
  const isAdmin = computed(() => ['admin', 'superadmin'].includes(user.value?.role ?? ''))

  async function login(username: string, password: string) {
    const { data } = await authApi.login(username, password)
    localStorage.setItem('access_token', data.access_token)
    localStorage.setItem('refresh_token', data.refresh_token)
    await fetchMe()
  }

  async function fetchMe() {
    try {
      loading.value = true
      const { data } = await authApi.me()
      user.value = data
    } finally {
      loading.value = false
    }
  }

  function logout() {
    user.value = null
    localStorage.removeItem('access_token')
    localStorage.removeItem('refresh_token')
  }

  async function changePassword(current: string, next: string) {
    await authApi.changePassword(current, next)
    if (user.value) user.value.force_pw_change = false
  }

  return { user, loading, isLoggedIn, isSuperAdmin, isAdmin, login, fetchMe, logout, changePassword }
})
