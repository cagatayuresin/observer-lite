<template>
  <div class="p-6">
    <h1 class="text-xl font-bold text-white mb-6">Audit Log</h1>
    <div class="card divide-y divide-surface-700">
      <div v-for="entry in log" :key="entry.id" class="flex items-start gap-4 px-4 py-2.5">
        <span class="text-xs text-slate-600 shrink-0 w-36">{{ fmtDate(entry.created_at) }}</span>
        <code class="text-xs text-brand-400 shrink-0 w-36 truncate">{{ entry.action }}</code>
        <span class="text-xs text-slate-400 truncate flex-1">{{ entry.detail || '' }}</span>
        <span class="text-xs text-slate-600 shrink-0">user #{{ entry.user_id ?? 'system' }}</span>
      </div>
      <div v-if="!log.length" class="px-4 py-12 text-center text-slate-500 text-sm">No audit log entries.</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { auditLogApi } from '@/api/index'

const log = ref<{ id: number; action: string; user_id: number | null; detail: string | null; created_at: string }[]>([])
onMounted(async () => { const { data } = await auditLogApi.list(); log.value = data })
function fmtDate(s: string) { return new Date(s).toLocaleString() }
</script>
