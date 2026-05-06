<template>
  <div class="p-6">
    <h1 class="text-xl font-bold text-white mb-6">Incidents</h1>
    <div class="flex gap-2 mb-4">
      <label class="flex items-center gap-2 cursor-pointer">
        <input type="checkbox" v-model="openOnly" @change="load" class="accent-brand-500" />
        <span class="text-sm text-slate-300">Open only</span>
      </label>
    </div>
    <div v-if="loading" class="text-slate-500 text-sm text-center py-12">Loading…</div>
    <div v-else-if="!incidents.length" class="text-slate-500 text-sm text-center py-12">No incidents.</div>
    <div v-else class="card divide-y divide-surface-700">
      <div v-for="inc in incidents" :key="inc.id" class="flex items-start gap-4 px-4 py-3">
        <span :class="inc.resolved_at ? 'badge-up' : 'badge-down'" class="mt-0.5 shrink-0">
          {{ inc.resolved_at ? 'Resolved' : 'Open' }}
        </span>
        <div class="flex-1 min-w-0">
          <RouterLink :to="`/monitors/${inc.monitor_id}`" class="text-sm font-medium text-white hover:text-brand-400">
            Monitor #{{ inc.monitor_id }}
          </RouterLink>
          <p class="text-xs text-slate-500 mt-0.5">{{ inc.root_cause || 'Unknown cause' }}</p>
          <p class="text-xs text-slate-600 mt-0.5">
            Started: {{ fmtDate(inc.started_at) }}
            <span v-if="inc.duration_seconds"> — Down for {{ fmtDur(inc.duration_seconds) }}</span>
            <span v-else-if="!inc.resolved_at"> — Still down</span>
          </p>
        </div>
        <button v-if="auth.isAdmin && !inc.acknowledged_at" class="btn-ghost text-xs" @click="acknowledge(inc)">Ack</button>
        <span v-if="inc.acknowledged_at" class="text-xs text-slate-600">Acked</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { RouterLink } from 'vue-router'
import { incidentsApi } from '@/api/index'
import { useAuthStore } from '@/stores/auth'

const auth = useAuthStore()
const incidents = ref<{ id: number; monitor_id: number; started_at: string; resolved_at: string | null; root_cause: string | null; duration_seconds: number | null; acknowledged_at: string | null }[]>([])
const loading = ref(false)
const openOnly = ref(false)

onMounted(load)

async function load() {
  loading.value = true
  const { data } = await incidentsApi.list({ open_only: openOnly.value })
  incidents.value = data
  loading.value = false
}

async function acknowledge(inc: { id: number; acknowledged_at: string | null }) {
  await incidentsApi.acknowledge(inc.id)
  inc.acknowledged_at = new Date().toISOString()
}

function fmtDate(s: string) { return new Date(s).toLocaleString() }
function fmtDur(s: number) {
  if (s < 60) return `${s}s`
  if (s < 3600) return `${Math.floor(s / 60)}m`
  return `${Math.floor(s / 3600)}h ${Math.floor((s % 3600) / 60)}m`
}
</script>
