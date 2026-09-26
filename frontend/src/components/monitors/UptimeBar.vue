<template>
  <div class="flex gap-0.5">
    <div
      v-for="(day, i) in days"
      :key="i"
      class="h-6 flex-1 rounded-sm cursor-pointer transition-opacity hover:opacity-80"
      :class="day.color"
      :title="`${day.date}: ${day.uptime}% uptime`"
    />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

interface DayStat { date: string; uptime: number }
const props = defineProps<{ data: DayStat[] }>()

function uptimeColor(uptime: number): string {
  if (uptime >= 99) return 'bg-emerald-500'
  if (uptime >= 90) return 'bg-amber-500'
  return 'bg-red-500'
}

const days = computed(() =>
  props.data.map((d) => ({
    ...d,
    color: uptimeColor(d.uptime),
  }))
)
</script>
