<template>
  <div class="p-6">
    <!-- Header -->
    <div class="flex items-center justify-between mb-6">
      <div>
        <h1 class="text-xl font-bold text-white">Dashboard</h1>
        <p class="text-slate-500 text-sm mt-0.5">{{ monitors.monitors.length }} monitors</p>
      </div>
      <RouterLink v-if="auth.isAdmin" to="/monitors/new" class="btn-primary">
        <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 4v16m8-8H4"/></svg>
        New Monitor
      </RouterLink>
    </div>

    <!-- Stats bar -->
    <div class="grid grid-cols-2 sm:grid-cols-4 gap-3 mb-6">
      <StatCard label="Up" :value="countByStatus('up')" color="emerald" />
      <StatCard label="Down" :value="countByStatus('down')" color="red" />
      <StatCard label="Warning" :value="countByStatus('warning')" color="amber" />
      <StatCard label="Paused" :value="countByStatus('paused') + countByStatus('pending')" color="slate" />
    </div>

    <!-- Filter bar -->
    <div class="flex flex-wrap gap-2 mb-4">
      <input
        v-model="monitors.search"
        @input="debouncedFetch"
        type="text"
        placeholder="Search monitors…"
        class="input w-56"
      />
      <button
        v-for="f in filters"
        :key="f.value"
        class="btn text-xs"
        :class="monitors.statusFilter === f.value ? 'btn-primary' : 'btn-ghost'"
        @click="setFilter(f.value)"
      >{{ f.label }}</button>
      <button v-if="auth.isAdmin && selectedIds.length" class="btn-danger text-xs ml-auto" @click="bulkDisable">
        Disable {{ selectedIds.length }}
      </button>
    </div>

    <!-- Monitor list -->
    <div v-if="monitors.loading" class="text-slate-500 text-sm py-12 text-center">Loading…</div>
    <div v-else-if="!monitors.monitors.length" class="text-slate-500 text-sm py-12 text-center">
      No monitors found.
      <RouterLink v-if="auth.isAdmin" to="/monitors/new" class="text-brand-400 hover:underline ml-1">Add one?</RouterLink>
    </div>
    <div v-else class="space-y-2">
      <MonitorRow
        v-for="m in monitors.monitors"
        :key="m.id"
        :monitor="m"
        :selected="selectedIds.includes(m.id)"
        @toggle-select="toggleSelect(m.id)"
        @pause="handlePause(m)"
        @resume="handleResume(m)"
        @delete="handleDelete(m)"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref, computed } from 'vue'
import { RouterLink } from 'vue-router'
import { useMonitorStore } from '@/stores/monitors'
import { useAuthStore } from '@/stores/auth'
import { useToastStore } from '@/stores/toast'
import { monitorsApi } from '@/api/monitors'
import MonitorRow from '@/components/monitors/MonitorRow.vue'
import StatCard from '@/components/monitors/StatCard.vue'
import type { Monitor } from '@/stores/monitors'

const monitors = useMonitorStore()
const auth = useAuthStore()
const toast = useToastStore()
const selectedIds = ref<number[]>([])

const filters = [
  { label: 'All', value: '' },
  { label: 'Up', value: 'up' },
  { label: 'Down', value: 'down' },
  { label: 'Warning', value: 'warning' },
  { label: 'Paused', value: 'paused' },
]

onMounted(() => monitors.fetchMonitors())

function countByStatus(s: string) {
  return monitors.monitors.filter((m) => m.current_status === s).length
}

let debounceTimer: ReturnType<typeof setTimeout>
function debouncedFetch() {
  clearTimeout(debounceTimer)
  debounceTimer = setTimeout(() => monitors.fetchMonitors(), 300)
}

function setFilter(v: string) {
  monitors.statusFilter = v
  monitors.fetchMonitors()
}

function toggleSelect(id: number) {
  const idx = selectedIds.value.indexOf(id)
  if (idx === -1) selectedIds.value.push(id)
  else selectedIds.value.splice(idx, 1)
}

async function handlePause(m: Monitor) {
  await monitorsApi.pause(m.id)
  m.current_status = 'paused'
  m.is_enabled = false
  toast.success(`${m.name} paused`)
}

async function handleResume(m: Monitor) {
  await monitorsApi.resume(m.id)
  m.current_status = 'pending'
  m.is_enabled = true
  toast.success(`${m.name} resumed`)
}

async function handleDelete(m: Monitor) {
  await monitorsApi.delete(m.id)
  monitors.monitors.splice(monitors.monitors.indexOf(m), 1)
  toast.success(`${m.name} deleted`)
}

async function bulkDisable() {
  await monitorsApi.bulk('disable', selectedIds.value)
  selectedIds.value.forEach((id) => {
    const m = monitors.monitors.find((m) => m.id === id)
    if (m) { m.is_enabled = false; m.current_status = 'paused' }
  })
  selectedIds.value = []
  toast.success('Monitors disabled')
}
</script>
