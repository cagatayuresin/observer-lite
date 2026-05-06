<template>
  <div class="p-6">
    <div class="flex items-center justify-between mb-6">
      <h1 class="text-xl font-bold text-white">Groups</h1>
      <button @click="openForm()" class="btn-primary">
        <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 4v16m8-8H4"/></svg>
        New Group
      </button>
    </div>

    <div v-if="loading" class="text-slate-500 text-sm py-12 text-center">Loading…</div>
    <div v-else-if="!groups.length" class="text-slate-500 text-sm py-12 text-center">No groups found.</div>
    <div v-else class="space-y-2">
      <div v-for="g in groups" :key="g.id" class="card px-4 py-3 flex items-center justify-between">
        <div class="flex-1">
          <p class="text-sm font-medium text-white">{{ g.name }}</p>
          <p v-if="g.description" class="text-xs text-slate-500">{{ g.description }}</p>
        </div>
        <div class="flex gap-2">
          <button @click="openForm(g)" class="btn-ghost px-2 py-1 text-xs">Edit</button>
          <button @click="handleDelete(g)" class="btn-ghost px-2 py-1 text-xs text-red-400 hover:text-red-300">Delete</button>
        </div>
      </div>
    </div>

    <!-- Group Form Modal -->
    <div v-if="showForm" class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
      <div class="bg-surface-800 rounded-xl shadow-xl w-full max-w-md border border-surface-700 flex flex-col max-h-[90vh]">
        <div class="px-4 py-3 border-b border-surface-700 shrink-0">
          <h3 class="text-base font-semibold text-white">{{ isEdit ? 'Edit Group' : 'New Group' }}</h3>
        </div>
        <div class="p-4 overflow-y-auto">
          <form id="group-form" @submit.prevent="saveGroup" class="space-y-4">
            <div>
              <label class="label">Name *</label>
              <input v-model="form.name" class="input" placeholder="Production" required />
            </div>
            <div>
              <label class="label">Description</label>
              <input v-model="form.description" class="input" placeholder="Critical services" />
            </div>
            
            <div v-if="allMonitors.length">
              <label class="label">Assign Monitors</label>
              <div class="space-y-1.5 max-h-48 overflow-y-auto p-2 bg-surface-900 border border-surface-700 rounded-lg">
                <label v-for="m in allMonitors" :key="m.id" class="flex items-center gap-2 p-1.5 hover:bg-surface-800 rounded cursor-pointer">
                  <input type="checkbox" :value="m.id" v-model="selectedMonitorIds" class="accent-brand-500 rounded bg-surface-700 border-surface-600 text-brand-500 focus:ring-brand-500/50" />
                  <span class="text-sm text-slate-300 truncate">{{ m.name }} <span class="text-xs text-slate-500">({{ m.url }})</span></span>
                </label>
              </div>
            </div>
          </form>
        </div>
        <div class="px-4 py-3 border-t border-surface-700 shrink-0 flex gap-3 bg-surface-800 rounded-b-xl">
          <button type="submit" form="group-form" class="btn-primary flex-1" :disabled="saving">Save</button>
          <button type="button" @click="showForm = false" class="btn-ghost flex-1">Cancel</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { groupsApi } from '@/api/index'
import { monitorsApi } from '@/api/monitors'
import { useToastStore } from '@/stores/toast'

const toast = useToastStore()
const groups = ref<{ id: number; name: string; description: string | null }[]>([])
const allMonitors = ref<any[]>([])
const loading = ref(true)
const saving = ref(false)

const showForm = ref(false)
const isEdit = ref(false)
const form = ref({ id: 0, name: '', description: '' })
const selectedMonitorIds = ref<number[]>([])
const originalMonitorIds = ref<number[]>([])

onMounted(load)

async function load() {
  loading.value = true
  try {
    const [{ data: gData }, { data: mData }] = await Promise.all([
      groupsApi.list(),
      monitorsApi.list()
    ])
    groups.value = gData
    allMonitors.value = mData
  } catch (e: any) {
    toast.error(e.response?.data?.detail || 'Failed to load data')
  } finally {
    loading.value = false
  }
}

function openForm(g?: { id: number; name: string; description: string | null }) {
  if (g) {
    isEdit.value = true
    form.value = { id: g.id, name: g.name, description: g.description || '' }
    const groupMonitors = allMonitors.value.filter(m => m.group_id === g.id).map(m => m.id)
    selectedMonitorIds.value = [...groupMonitors]
    originalMonitorIds.value = [...groupMonitors]
  } else {
    isEdit.value = false
    form.value = { id: 0, name: '', description: '' }
    selectedMonitorIds.value = []
    originalMonitorIds.value = []
  }
  showForm.value = true
}

async function saveGroup() {
  saving.value = true
  try {
    const payload = {
      name: form.value.name,
      description: form.value.description || null,
    }
    
    let groupId = form.value.id
    if (isEdit.value) {
      await groupsApi.update(groupId, payload)
      toast.success('Group updated')
    } else {
      const { data } = await groupsApi.create(payload)
      groupId = data.id
      toast.success('Group created')
    }

    // Update monitors
    const toAdd = selectedMonitorIds.value.filter(id => !originalMonitorIds.value.includes(id))
    const toRemove = originalMonitorIds.value.filter(id => !selectedMonitorIds.value.includes(id))
    
    const tasks = []
    for (const id of toAdd) tasks.push(monitorsApi.patch(id, { group_id: groupId }))
    for (const id of toRemove) tasks.push(monitorsApi.patch(id, { group_id: null }))
    
    if (tasks.length) {
      await Promise.all(tasks)
    }

    showForm.value = false
    await load()
  } catch (e: any) {
    toast.error(e.response?.data?.detail || 'Failed to save group')
  } finally {
    saving.value = false
  }
}

async function handleDelete(g: any) {
  if (!confirm(`Delete group "${g.name}"?`)) return
  try {
    await groupsApi.delete(g.id)
    toast.success('Group deleted')
    groups.value = groups.value.filter(x => x.id !== g.id)
    await load() // refresh monitors too
  } catch (e: any) {
    toast.error(e.response?.data?.detail || 'Failed to delete')
  }
}
</script>
