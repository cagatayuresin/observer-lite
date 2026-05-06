<template>
  <div class="min-h-screen flex items-center justify-center bg-surface-900 p-4">
    <div class="w-full max-w-sm">
      <div class="card p-6">
        <h2 class="text-lg font-semibold text-white mb-1">Change Password</h2>
        <p class="text-slate-400 text-sm mb-6">You must set a new password before continuing.</p>
        <form @submit.prevent="handleSubmit">
          <div class="mb-4">
            <label class="label">Current Password</label>
            <input v-model="form.current" type="password" class="input" required />
          </div>
          <div class="mb-4">
            <label class="label">New Password</label>
            <input v-model="form.next" type="password" class="input" minlength="8" required />
          </div>
          <div class="mb-6">
            <label class="label">Confirm New Password</label>
            <input v-model="form.confirm" type="password" class="input" required />
          </div>
          <p v-if="error" class="text-red-400 text-sm mb-4">{{ error }}</p>
          <button type="submit" class="btn-primary w-full justify-center" :disabled="loading">
            {{ loading ? 'Saving…' : 'Set New Password' }}
          </button>
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
const form = ref({ current: '', next: '', confirm: '' })
const loading = ref(false)
const error = ref('')

async function handleSubmit() {
  error.value = ''
  if (form.value.next !== form.value.confirm) { error.value = 'Passwords do not match'; return }
  if (form.value.next.length < 8) { error.value = 'Password must be at least 8 characters'; return }
  loading.value = true
  try {
    await auth.changePassword(form.value.current, form.value.next)
    router.push('/dashboard')
  } catch (e: unknown) {
    error.value = (e as { response?: { data?: { detail?: string } } })?.response?.data?.detail ?? 'Failed to change password'
  } finally {
    loading.value = false
  }
}
</script>
