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
            <div class="space-y-4">
              <div>
                <label class="label">Start Date & Time</label>
                <div class="flex gap-2">
                  <input v-model="form.start_date" type="date" class="input flex-1" />
                  <input v-model="form.start_time" type="time" class="input w-32" />
                </div>
              </div>
              <div>
                <label class="label">End Date & Time</label>
                <div class="flex gap-2">
                  <input v-model="form.end_date" type="date" class="input flex-1" />
                  <input v-model="form.end_time" type="time" class="input w-32" />
                </div>
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
const form = ref({ name: '', start_date: '', start_time: '00:00', end_date: '', end_time: '00:00', suppress_alerts: true })

onMounted(load)
async function load() {
  const { data } = await maintenanceApi.list()
  windows.value = data
}
function openForm() { form.value = { name: '', start_date: '', start_time: '00:00', end_date: '', end_time: '00:00', suppress_alerts: true }; showForm.value = true }
function isActive(w: typeof windows.value[number]) {
  const now = Date.now()
  return new Date(w.starts_at).getTime() <= now && now <= new Date(w.ends_at).getTime()
}
async function saveWindow() {
  if (!form.value.name || !form.value.start_date || !form.value.end_date || !form.value.start_time || !form.value.end_time) {
    toast.error('Lütfen isim, başlangıç ve bitiş zamanını girin')
    return
  }
  const startsAt = new Date(`${form.value.start_date}T${form.value.start_time}:00`)
  const endsAt = new Date(`${form.value.end_date}T${form.value.end_time}:00`)
  
  if (isNaN(startsAt.getTime()) || isNaN(endsAt.getTime())) {
    toast.error('Geçersiz tarih formatı')
    return
  }

  if (startsAt >= endsAt) {
    toast.error('Bitiş zamanı başlangıçtan sonra olmalıdır')
    return
  }

  try {
    await maintenanceApi.create({ 
      name: form.value.name,
      suppress_alerts: form.value.suppress_alerts,
      starts_at: startsAt.toISOString(), 
      ends_at: endsAt.toISOString(),
      monitor_ids: []
    })
    toast.success('Maintenance window created')
    showForm.value = false
    await load()
  } catch (e: any) {
    toast.error('Oluşturulamadı')
  }
}
async function deleteWindow(id: number) {
  await maintenanceApi.delete(id)
  windows.value = windows.value.filter((w) => w.id !== id)
}
function fmtDate(s: string) { return new Date(s).toLocaleString() }
</script>
