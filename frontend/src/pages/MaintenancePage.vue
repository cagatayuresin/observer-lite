<template>
  <div class="p-6 max-w-3xl mx-auto">
    <div class="flex items-center justify-between mb-6">
      <h1 class="text-xl font-bold text-white">Maintenance Windows</h1>
      <button class="btn-primary" @click="openForm()">+ New Window</button>
    </div>
    <div v-if="!windows.length" class="text-slate-500 text-sm text-center py-12">No maintenance windows.</div>
    <div v-else class="card divide-y divide-surface-700">
      <div v-for="w in windows" :key="w.id" class="flex items-start gap-4 px-4 py-3">
        <div class="flex-1">
          <p class="text-sm font-medium text-white">{{ w.name }}</p>
          <p class="text-xs text-slate-500">{{ fmtDate(w.starts_at) }} → {{ fmtDate(w.ends_at) }}</p>
          <p class="text-xs text-slate-600">{{ w.monitor_ids.length }} monitors</p>
        </div>
        <span :class="isActive(w) ? 'badge-warning' : 'badge-paused'">{{ isActive(w) ? 'Active' : 'Inactive' }}</span>
        <button class="btn-ghost text-xs text-red-400" @click="deleteWindow(w.id)">Delete</button>
      </div>
    </div>

    <Teleport to="body">
      <div v-if="showForm" class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60">
        <div class="card w-full max-w-lg p-6">
          <h2 class="text-lg font-semibold text-white mb-4">New Maintenance Window</h2>
          <div class="space-y-4">
            <div>
              <label class="label">Name</label>
              <input v-model="form.name" class="input" placeholder="Planned maintenance" />
            </div>
            <div class="grid grid-cols-2 gap-3">
              <div>
                <label class="label">Start</label>
                <input v-model="form.starts_at" type="datetime-local" class="input" />
              </div>
              <div>
                <label class="label">End</label>
                <input v-model="form.ends_at" type="datetime-local" class="input" />
              </div>
            </div>
            <label class="flex items-center gap-2 cursor-pointer">
              <input type="checkbox" v-model="form.suppress_alerts" class="accent-brand-500" />
              <span class="text-sm text-slate-300">Suppress alerts during window</span>
            </label>
          </div>
          <div class="flex gap-3 mt-6">
            <button class="btn-primary" @click="saveWindow">Create</button>
            <button class="btn-ghost" @click="showForm = false">Cancel</button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { maintenanceApi } from '@/api/index'
import { useToastStore } from '@/stores/toast'

const toast = useToastStore()
const windows = ref<{ id: number; name: string; starts_at: string; ends_at: string; monitor_ids: number[]; suppress_alerts: boolean }[]>([])
const showForm = ref(false)
const form = ref({ name: '', starts_at: '', ends_at: '', suppress_alerts: true })

onMounted(load)
async function load() {
  const { data } = await maintenanceApi.list()
  windows.value = data
}
function openForm() { form.value = { name: '', starts_at: '', ends_at: '', suppress_alerts: true }; showForm.value = true }
function isActive(w: typeof windows.value[number]) {
  const now = Date.now()
  return new Date(w.starts_at).getTime() <= now && now <= new Date(w.ends_at).getTime()
}
async function saveWindow() {
  await maintenanceApi.create({ ...form.value, starts_at: new Date(form.value.starts_at).toISOString(), ends_at: new Date(form.value.ends_at).toISOString() })
  toast.success('Maintenance window created')
  showForm.value = false
  await load()
}
async function deleteWindow(id: number) {
  await maintenanceApi.delete(id)
  windows.value = windows.value.filter((w) => w.id !== id)
}
function fmtDate(s: string) { return new Date(s).toLocaleString() }
</script>
