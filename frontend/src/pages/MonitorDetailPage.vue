<template>
  <div class="p-6 max-w-4xl mx-auto" v-if="monitor">
    <!-- Header -->
    <div class="flex items-start gap-4 mb-6">
      <RouterLink to="/dashboard" class="btn-ghost px-2 py-1 mt-0.5">← Back</RouterLink>
      <div class="flex-1">
        <div class="flex items-center gap-3 flex-wrap">
          <h1 class="text-xl font-bold text-white">{{ monitor.name }}</h1>
          <StatusBadge :status="monitor.current_status" />
        </div>
        <p class="text-slate-500 text-sm mt-0.5 break-all">{{ monitor.url }}</p>
      </div>
      <div class="flex gap-2">
        <button class="btn-ghost text-sm" @click="checkNow" :disabled="checking">
          {{ checking ? 'Checking…' : 'Check Now' }}
        </button>
        <RouterLink :to="`/monitors/${monitor.id}/edit`" class="btn-ghost text-sm">Edit</RouterLink>
      </div>
    </div>

    <!-- Stats -->
    <div class="grid grid-cols-2 md:grid-cols-4 gap-3 mb-6">
      <div class="card px-4 py-3 text-center">
        <p class="text-2xl font-bold" :class="(stats?.uptime_percent ?? 0) >= 99 ? 'text-emerald-400' : 'text-amber-400'">
          {{ stats ? `${stats.uptime_percent}%` : '—' }}
        </p>
        <p class="text-xs text-slate-500">Uptime ({{ statDays }}d)</p>
      </div>
      <div class="card px-4 py-3 text-center">
        <p class="text-2xl font-bold text-slate-200">
          {{ monitor.last_response_time_ms != null ? `${monitor.last_response_time_ms}ms` : '—' }}
        </p>
        <p class="text-xs text-slate-500">Last Response</p>
      </div>
      <div class="card px-4 py-3 text-center">
        <p class="text-2xl font-bold text-slate-200">{{ stats?.total_incidents ?? '—' }}</p>
        <p class="text-xs text-slate-500">Incidents ({{ statDays }}d)</p>
      </div>
      <div class="card px-4 py-3 text-center">
        <p class="text-2xl font-bold text-slate-200">
          {{ stats?.avg_response_time_ms != null ? `${Math.round(stats.avg_response_time_ms)}ms` : '—' }}
        </p>
        <p class="text-xs text-slate-500">Avg Response</p>
      </div>
    </div>

    <!-- Response time chart -->
    <div class="card p-4 mb-6">
      <div class="flex items-center justify-between mb-3">
        <h2 class="text-sm font-semibold text-white">Response Time (24h)</h2>
        <select v-model="statDays" @change="loadStats" class="input w-28 text-xs py-1">
          <option :value="1">1 day</option>
          <option :value="7">7 days</option>
          <option :value="30">30 days</option>
          <option :value="90">90 days</option>
        </select>
      </div>
      <ResponseTimeChart v-if="history.length" :data="history" />
      <p v-else class="text-slate-600 text-sm text-center py-8">No data yet</p>
    </div>

    <!-- Uptime bar -->
    <div class="card p-4 mb-6" v-if="uptimeBarData.length">
      <h2 class="text-sm font-semibold text-white mb-3">90-day Uptime</h2>
      <UptimeBar :data="uptimeBarData" />
    </div>

    <!-- Incidents -->
    <div class="card p-4">
      <h2 class="text-sm font-semibold text-white mb-3">Recent Incidents</h2>
      <div v-if="!incidents.length" class="text-slate-600 text-sm py-4 text-center">No incidents</div>
      <div v-else class="space-y-2">
        <div v-for="inc in incidents" :key="inc.id" class="flex items-start gap-3 py-2 border-b border-surface-700 last:border-0">
          <span :class="inc.resolved_at ? 'badge-up' : 'badge-down'" class="mt-0.5 shrink-0">
            {{ inc.resolved_at ? 'Resolved' : 'Open' }}
          </span>
          <div class="flex-1 min-w-0">
            <p class="text-xs text-slate-400">{{ inc.root_cause || 'Unknown cause' }}</p>
            <p class="text-xs text-slate-600 mt-0.5">
              {{ fmtDate(inc.started_at) }}
              <span v-if="inc.duration_seconds"> — {{ fmtDuration(inc.duration_seconds) }}</span>
            </p>
          </div>
        </div>
      </div>
    </div>
  </div>
  <div v-else class="p-6 text-slate-500 text-center">Loading…</div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { RouterLink, useRoute } from 'vue-router'
import { monitorsApi } from '@/api/monitors'
import StatusBadge from '@/components/monitors/StatusBadge.vue'
import ResponseTimeChart from '@/components/monitors/ResponseTimeChart.vue'
import UptimeBar from '@/components/monitors/UptimeBar.vue'
import { useToastStore } from '@/stores/toast'
import type { Monitor } from '@/stores/monitors'

const route = useRoute()
const toast = useToastStore()
const monitorId = Number(route.params.id)

const monitor = ref<Monitor | null>(null)
const stats = ref<{ uptime_percent: number; avg_response_time_ms: number; total_incidents: number } | null>(null)
const history = ref<{ checked_at: string; response_time_ms: number | null }[]>([])
const incidents = ref<{ id: number; started_at: string; resolved_at: string | null; root_cause: string | null; duration_seconds: number | null }[]>([])
const uptimeBarData = ref<{ date: string; uptime: number }[]>([])
const statDays = ref(30)
const checking = ref(false)

onMounted(async () => {
  const [monResp, incResp] = await Promise.all([
    monitorsApi.get(monitorId),
    monitorsApi.incidents(monitorId),
  ])
  monitor.value = monResp.data
  incidents.value = incResp.data
  await loadStats()
  await loadHistory()
})

async function loadStats() {
  const resp = await monitorsApi.stats(monitorId, statDays.value)
  stats.value = resp.data
}

async function loadHistory() {
  const since = new Date(Date.now() - 24 * 3600 * 1000).toISOString()
  const resp = await monitorsApi.history(monitorId, { from: since, limit: 500 })
  history.value = resp.data.reverse()
}

async function checkNow() {
  checking.value = true
  try {
    await monitorsApi.checkNow(monitorId)
    toast.info('Check triggered')
    setTimeout(async () => {
      const resp = await monitorsApi.get(monitorId)
      monitor.value = resp.data
      await loadHistory()
      checking.value = false
    }, 5000)
  } catch {
    checking.value = false
  }
}

function fmtDate(iso: string) {
  return new Date(iso).toLocaleString()
}

function fmtDuration(s: number) {
  if (s < 60) return `${s}s`
  if (s < 3600) return `${Math.floor(s / 60)}m ${s % 60}s`
  return `${Math.floor(s / 3600)}h ${Math.floor((s % 3600) / 60)}m`
}
</script>
