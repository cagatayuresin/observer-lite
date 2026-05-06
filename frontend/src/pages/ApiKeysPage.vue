<template>
  <div class="p-6 max-w-3xl mx-auto">
    <div class="flex items-center justify-between mb-6">
      <h1 class="text-xl font-bold text-white">API Keys</h1>
      <button class="btn-primary" @click="showCreate = true">+ New Key</button>
    </div>
    <div class="card divide-y divide-surface-700">
      <div v-for="k in keys" :key="k.id" class="flex items-center gap-4 px-4 py-3">
        <div class="flex-1">
          <p class="text-sm font-medium text-white">{{ k.name }}</p>
          <p class="text-xs text-slate-500 font-mono">{{ k.key_prefix }}…</p>
          <p class="text-xs text-slate-600">Last used: {{ k.last_used_at ? new Date(k.last_used_at).toLocaleDateString() : 'Never' }}</p>
        </div>
        <span v-if="k.expires_at" class="text-xs text-slate-500">Expires {{ new Date(k.expires_at).toLocaleDateString() }}</span>
        <button class="btn-ghost text-xs text-red-400" @click="deleteKey(k.id)">Revoke</button>
      </div>
      <div v-if="!keys.length" class="px-4 py-12 text-center text-slate-500 text-sm">No API keys yet.</div>
    </div>

    <!-- New key modal -->
    <Teleport to="body">
      <div v-if="showCreate" class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60">
        <div class="card w-full max-w-md p-6">
          <h2 class="text-lg font-semibold text-white mb-4">New API Key</h2>
          <div class="space-y-4">
            <div>
              <label class="label">Name</label>
              <input v-model="newName" class="input" placeholder="CI/CD pipeline" />
            </div>
          </div>
          <div class="flex gap-3 mt-6">
            <button class="btn-primary" @click="createKey">Generate</button>
            <button class="btn-ghost" @click="showCreate = false">Cancel</button>
          </div>
        </div>
      </div>
      <!-- Show raw key once -->
      <div v-if="createdKey" class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60">
        <div class="card w-full max-w-md p-6">
          <h2 class="text-lg font-semibold text-white mb-2">Save your API key</h2>
          <p class="text-slate-400 text-sm mb-4">This is the only time you'll see this key.</p>
          <code class="block bg-surface-900 rounded-lg px-3 py-2 text-brand-400 text-xs font-mono break-all">{{ createdKey }}</code>
          <button class="btn-primary w-full justify-center mt-4" @click="createdKey = ''">Done</button>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { apiKeysApi } from '@/api/index'
import { useToastStore } from '@/stores/toast'

const toast = useToastStore()
const keys = ref<{ id: number; name: string; key_prefix: string; last_used_at: string | null; expires_at: string | null }[]>([])
const showCreate = ref(false)
const newName = ref('')
const createdKey = ref('')

onMounted(load)
async function load() { const { data } = await apiKeysApi.list(); keys.value = data }
async function createKey() {
  const { data } = await apiKeysApi.create({ name: newName.value })
  createdKey.value = data.raw_key
  showCreate.value = false
  newName.value = ''
  await load()
}
async function deleteKey(id: number) {
  await apiKeysApi.delete(id)
  keys.value = keys.value.filter((k) => k.id !== id)
  toast.success('API key revoked')
}
</script>
