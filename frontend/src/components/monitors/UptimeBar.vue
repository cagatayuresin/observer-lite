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

const days = computed(() =>
  props.data.map((d) => ({
    ...d,
    color: d.uptime >= 99 ? 'bg-emerald-500' : d.uptime >= 90 ? 'bg-amber-500' : 'bg-red-500',
  }))
)
</script>
