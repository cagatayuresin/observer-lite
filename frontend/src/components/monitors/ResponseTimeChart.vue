<template>
  <div ref="container" class="w-full h-40" />
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'
import uPlot from 'uplot'
import 'uplot/dist/uPlot.min.css'

interface DataPoint { checked_at: string; response_time_ms: number | null }
const props = defineProps<{ data: DataPoint[] }>()
const container = ref<HTMLElement | null>(null)
let chart: uPlot | null = null

function build() {
  if (!container.value || !props.data.length) return
  chart?.destroy()

  const timestamps = props.data.map((d) => new Date(d.checked_at).getTime() / 1000)
  const values = props.data.map((d) => d.response_time_ms ?? null)

  chart = new uPlot({
    width: container.value.clientWidth,
    height: 160,
    series: [
      {},
      {
        label: 'Response (ms)',
        stroke: '#3b82f6',
        fill: 'rgba(59,130,246,0.1)',
        width: 1.5,
      },
    ],
    axes: [
      { stroke: '#475569', ticks: { stroke: '#334155' }, grid: { stroke: '#1e293b' } },
      { stroke: '#475569', ticks: { stroke: '#334155' }, grid: { stroke: '#1e293b' } },
    ],
    scales: { x: { time: true } },
    cursor: { drag: { x: true } },
  }, [timestamps, values], container.value)
}

onMounted(build)
watch(() => props.data, build)
onUnmounted(() => chart?.destroy())
</script>
