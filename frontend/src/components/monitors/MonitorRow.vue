<template>
  <div
    class="card px-4 py-3 flex items-center gap-4 hover:border-surface-600 transition-colors cursor-pointer"
    @click="router.push(`/monitors/${monitor.id}`)"
  >
    <!-- Checkbox -->
    <input
      type="checkbox"
      :checked="selected"
      @click.stop="emit('toggle-select')"
      class="w-4 h-4 accent-brand-500 shrink-0"
    />

    <!-- Status dot -->
    <StatusBadge :status="monitor.current_status" />

    <!-- Name & URL -->
    <div class="flex-1 min-w-0">
      <p class="text-sm font-medium text-white truncate">{{ monitor.name }}</p>
      <p class="text-xs text-slate-500 truncate">{{ monitor.url }}</p>
    </div>

    <!-- Response time -->
    <div class="text-right hidden sm:block w-20 shrink-0">
      <p class="text-sm font-medium" :class="rtColor">
        {{ monitor.last_response_time_ms != null ? `${monitor.last_response_time_ms}ms` : '—' }}
      </p>
      <p class="text-xs text-slate-600">response</p>
    </div>

    <!-- Interval -->
    <div class="text-right hidden md:block w-16 shrink-0">
      <p class="text-xs text-slate-500">every {{ fmtInterval(monitor.check_interval_seconds) }}</p>
    </div>

    <!-- History Sparkline -->
    <div class="hidden lg:flex items-center gap-0.5 w-40 shrink-0 h-5" title="Recent checks">
      <div
        v-for="h in history"
        :key="h.id"
        class="flex-1 h-full rounded-sm opacity-80 hover:opacity-100"
        :class="h.status === 'up' ? 'bg-emerald-500' : 'bg-red-500'"
      ></div>
      <div v-if="!history.length" class="text-xs text-slate-600">No data</div>
    </div>

    <!-- Actions -->
    <div class="flex gap-1 shrink-0" @click.stop>
      <button
        v-if="auth.isAdmin && monitor.is_enabled"
        @click="emit('pause')"
        class="btn-ghost px-2 py-1 text-xs"
        title="Pause"
      >⏸</button>
      <button
        v-if="auth.isAdmin && !monitor.is_enabled"
        @click="emit('resume')"
        class="btn-ghost px-2 py-1 text-xs"
        title="Resume"
      >▶</button>
      <RouterLink
        v-if="auth.isAdmin"
        :to="`/monitors/${monitor.id}/edit`"
        class="btn-ghost px-2 py-1 text-xs"
        title="Edit"
      >✎</RouterLink>
      <button
        v-if="auth.isAdmin"
        @click="confirmDelete = true"
        class="btn-ghost px-2 py-1 text-xs text-red-400 hover:text-red-300"
        title="Delete"
      >✕</button>
    </div>
  </div>

  <ConfirmDialog
    v-model="confirmDelete"
    title="Delete Monitor"
    :message="`Delete '${monitor.name}'? This will remove all check history and incidents.`"
    confirm-label="Delete"
    :danger="true"
    @confirm="emit('delete')"
  />
</template>

<script setup lang="ts">
import { computed, ref, onMounted } from 'vue'
import { RouterLink, useRouter } from 'vue-router'
import type { Monitor } from '@/stores/monitors'
import { useAuthStore } from '@/stores/auth'
import { monitorsApi } from '@/api/monitors'
import StatusBadge from './StatusBadge.vue'
import ConfirmDialog from '@/components/common/ConfirmDialog.vue'

const props = defineProps<{ monitor: Monitor; selected: boolean }>()
const emit = defineEmits<{ 'toggle-select': []; pause: []; resume: []; delete: [] }>()
const auth = useAuthStore()
const router = useRouter()
const confirmDelete = ref(false)
const history = ref<{ id: number; status: string }[]>([])

onMounted(async () => {
  if (props.monitor.current_status !== 'pending') {
    try {
      const { data } = await monitorsApi.history(props.monitor.id, { limit: 40 })
      history.value = data.reverse()
    } catch {
      // ignore
    }
  }
})

const rtColor = computed(() => {
  const rt = props.monitor.last_response_time_ms
  if (rt == null) return 'text-slate-600'
  if (rt > props.monitor.response_time_warning_ms) return 'text-amber-400'
  return 'text-slate-300'
})

function fmtInterval(s: number) {
  if (s < 60) return `${s}s`
  if (s < 3600) return `${s / 60}m`
  return `${s / 3600}h`
}
</script>
