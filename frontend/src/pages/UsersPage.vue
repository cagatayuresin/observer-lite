<template>
  <div class="p-6 max-w-3xl mx-auto">
    <div class="flex items-center justify-between mb-6">
      <h1 class="text-xl font-bold text-white">Users</h1>
      <button class="btn-primary" @click="openForm()">+ New User</button>
    </div>
    <div class="card divide-y divide-surface-700">
      <div v-for="u in users" :key="u.id" class="flex items-center gap-4 px-4 py-3">
        <div class="w-8 h-8 rounded-full bg-brand-600/30 flex items-center justify-center text-sm font-bold text-brand-400">
          {{ u.username[0].toUpperCase() }}
        </div>
        <div class="flex-1">
          <p class="text-sm font-medium text-white">{{ u.username }}</p>
          <p class="text-xs text-slate-500">{{ u.email }}</p>
        </div>
        <span class="text-xs text-slate-400 capitalize px-2 py-0.5 rounded bg-surface-700">{{ u.role }}</span>
        <span :class="u.is_active ? 'badge-up' : 'badge-paused'">{{ u.is_active ? 'Active' : 'Inactive' }}</span>
        <button class="btn-ghost text-xs" @click="openForm(u)">Edit</button>
        <button class="btn-ghost text-xs" @click="resetPw(u.id)">Reset PW</button>
        <button v-if="u.id !== auth.user?.id" class="btn-ghost text-xs text-red-400" @click="deleteUser(u.id)">Delete</button>
      </div>
    </div>

    <Teleport to="body">
      <div v-if="showForm" class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60">
        <div class="card w-full max-w-md p-6">
          <h2 class="text-lg font-semibold text-white mb-4">{{ editing ? 'Edit User' : 'New User' }}</h2>
          <div class="space-y-4">
            <div v-if="!editing">
              <label class="label">Username</label>
              <input v-model="form.username" class="input" />
            </div>
            <div v-if="!editing">
              <label class="label">Password</label>
              <input v-model="form.password" type="password" class="input" />
            </div>
            <div>
              <label class="label">Email</label>
              <input v-model="form.email" type="email" class="input" />
            </div>
            <div>
              <label class="label">Role</label>
              <select v-model="form.role" class="input">
                <option value="viewer">Viewer</option>
                <option value="admin">Admin</option>
                <option value="superadmin">Super Admin</option>
              </select>
            </div>
            <label v-if="editing" class="flex items-center gap-2 cursor-pointer">
              <input type="checkbox" v-model="form.is_active" class="accent-brand-500" />
              <span class="text-sm text-slate-300">Active</span>
            </label>
          </div>
          <div class="flex gap-3 mt-6">
            <button class="btn-primary" @click="saveUser">{{ editing ? 'Save' : 'Create' }}</button>
            <button class="btn-ghost" @click="showForm = false">Cancel</button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { usersApi } from '@/api/index'
import { useAuthStore } from '@/stores/auth'
import { useToastStore } from '@/stores/toast'

const auth = useAuthStore()
const toast = useToastStore()
const users = ref<{ id: number; username: string; email: string; role: string; is_active: boolean }[]>([])
const showForm = ref(false)
const editing = ref<number | null>(null)
const form = ref({ username: '', email: '', password: '', role: 'viewer', is_active: true })

onMounted(load)

async function load() {
  const { data } = await usersApi.list()
  users.value = data
}

function openForm(u?: typeof users.value[number]) {
  editing.value = u?.id ?? null
  form.value = { username: u?.username ?? '', email: u?.email ?? '', password: '', role: u?.role ?? 'viewer', is_active: u?.is_active ?? true }
  showForm.value = true
}

async function saveUser() {
  if (editing.value) {
    await usersApi.update(editing.value, { email: form.value.email, role: form.value.role, is_active: form.value.is_active })
    toast.success('User updated')
  } else {
    await usersApi.create({ username: form.value.username, email: form.value.email, password: form.value.password, role: form.value.role })
    toast.success('User created')
  }
  showForm.value = false
  await load()
}

async function deleteUser(id: number) {
  await usersApi.delete(id)
  users.value = users.value.filter((u) => u.id !== id)
  toast.success('User deleted')
}

async function resetPw(id: number) {
  await usersApi.resetPassword(id)
  toast.success('Password reset to "password123" — user must change on next login')
}
</script>
