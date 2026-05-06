<template>
  <div class="min-h-screen flex items-center justify-center bg-surface-900 p-4">
    <div class="w-full max-w-sm">
      <!-- Logo -->
      <div class="text-center mb-8">
        <div class="inline-flex items-center justify-center w-14 h-14 rounded-2xl bg-brand-600/20 mb-4">
          <svg class="w-8 h-8 text-brand-500" fill="currentColor" viewBox="0 0 24 24">
            <circle cx="12" cy="12" r="10" opacity=".15"/>
            <circle cx="12" cy="12" r="6" opacity=".3"/>
            <circle cx="12" cy="12" r="2.5"/>
          </svg>
        </div>
        <h1 class="text-2xl font-bold text-white">Observer Lite</h1>
        <p class="text-slate-500 text-sm mt-1">Uptime monitoring, simplified</p>
      </div>

      <div class="card p-6">
        <form @submit.prevent="handleLogin">
          <div class="mb-4">
            <label class="label">Username</label>
            <input v-model="form.username" type="text" class="input" placeholder="admin" autocomplete="username" required />
          </div>
          <div class="mb-6">
            <label class="label">Password</label>
            <input v-model="form.password" type="password" class="input" placeholder="••••••••" autocomplete="current-password" required />
          </div>
          <button type="submit" class="btn-primary w-full justify-center" :disabled="loading">
            <span v-if="loading">Signing in…</span>
            <span v-else>Sign In</span>
          </button>
          <p v-if="error" class="text-red-400 text-sm mt-3 text-center">{{ error }}</p>
        </form>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

const auth = useAuthStore()
const router = useRouter()
const form = ref({ username: '', password: '' })
const loading = ref(false)
const error = ref('')

async function handleLogin() {
  error.value = ''
  loading.value = true
  try {
    await auth.login(form.value.username, form.value.password)
    if (auth.user?.force_pw_change) {
      router.push('/change-password')
    } else {
      router.push('/dashboard')
    }
  } catch (e: unknown) {
    error.value = (e as { response?: { data?: { detail?: string } } })?.response?.data?.detail ?? 'Login failed'
  } finally {
    loading.value = false
  }
}
</script>
