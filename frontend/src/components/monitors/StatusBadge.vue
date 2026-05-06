<template>
  <span :class="badgeClass">
    <span class="w-1.5 h-1.5 rounded-full" :class="dotClass" />
    {{ label }}
  </span>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{ status: string }>()

const config: Record<string, { badge: string; dot: string; label: string }> = {
  up:      { badge: 'badge-up',      dot: 'bg-emerald-400', label: 'Up' },
  down:    { badge: 'badge-down',    dot: 'bg-red-400',     label: 'Down' },
  warning: { badge: 'badge-warning', dot: 'bg-amber-400',   label: 'Warning' },
  paused:  { badge: 'badge-paused',  dot: 'bg-slate-400',   label: 'Paused' },
  pending: { badge: 'badge-pending', dot: 'bg-slate-600',   label: 'Pending' },
}

const cfg = computed(() => config[props.status] ?? config.pending)
const badgeClass = computed(() => cfg.value.badge)
const dotClass = computed(() => cfg.value.dot)
const label = computed(() => cfg.value.label)
</script>
